# AWS S3 + CloudFront — Astrophoto images

Runbook for provisioning the photo-storage and CDN infrastructure for
the Astrophoto Phase 1 (Photographer Showcase) deployment.

**Architecture pivot:** commit `1837d0a` documented a CloudFront →
Lambda Function URL pattern. That pattern was invalidated in this AWS
account because Lambda Function URL Block Public Access (FUBPA) — a
feature AWS rolled out in late 2024 — silently blocks public access to
Function URLs, breaking unauthenticated CloudFront→Lambda invocations
even when `AuthType: NONE`. The confirmed-working pattern, validated
end-to-end against real AWS in the staging deployment, is
**CloudFront + Lambda@Edge (origin-request)**. This runbook reflects
that architecture.

---

## Overview

Each photo produces two objects in S3:

- `originals/<photo-id>.<ext>` — file as uploaded (JPEG / PNG / TIFF,
  up to 200 MB). Written once; never served directly to browsers.
- `display/<photo-id>.jpg` — **display master**: max 4096 px on the
  long edge, q=85 baseline JPEG, sRGB, EXIF and ICC stripped. This is
  the image the CDN transforms on demand.

The display master is derived once at upload finalize time by the
backend's `spawn_blocking` pipeline. Every browser request afterwards
fetches the display master through the Lambda transformer, not the raw
original. This decouples the CDN resizer from heavy source formats
(16-bit TIFF at 200 MB), cuts Lambda memory / cold-start cost, and
keeps transform logic simple.

**Traffic path:**

```
Browser
  └─► CloudFront  (ddo5booq71gbx.cloudfront.net in staging)
        ├── Lambda@Edge (origin-request)
        │     ├── matches /img/<photo-id>
        │     ├── fetches display/<photo-id>.jpg from S3 via execution role
        │     ├── sharp-transforms per query params (w, h, fit, q, fm)
        │     └── returns custom response; CloudFront caches per cache-key
        └── S3 origin (OAC, signing service "s3")
              └── fallback: non-/img/* paths passed through to S3 directly
```

CloudFront caches responses keyed on the query string (`w`, `h`,
`fit`, `q`, `fm`). On a cache miss the Lambda@Edge function fires as
an origin-request trigger, fetches the display master, transforms it,
and returns a synthetic response. The backend
(`astrophoto-staging-uploader` / `astrophoto-prod-uploader` IAM user)
writes both `originals/` and `display/` directly via `PutObject`.
Browsers never reach S3 — all read traffic goes through CloudFront.

---

## Architecture: why Lambda@Edge, not Function URL

**Lambda Function URL Block Public Access (FUBPA)** was rolled out by
AWS in 2024-Q4. In accounts where FUBPA is active, Function URLs with
`AuthType: NONE` are silently blocked. CloudFront→Lambda Function URL
with OAC (`AuthType: AWS_IAM`) is the documented workaround, but
requires `lambda:InvokeFunctionUrl` permission scoped to the
distribution — a circular dependency: you need the distribution ARN
before creating it, and the permission before CloudFront can route to
it. In practice, FUBPA + IAM auth on Function URLs adds fragility
without meaningful security benefit over Lambda@Edge + S3 OAC.

**Lambda@Edge (origin-request)** is the recommended pattern for
CloudFront image transforms:

- The Lambda runs at CloudFront edge locations as part of the request
  lifecycle — no separate origin to secure.
- S3 access is via execution role + S3 OAC bucket policy; no Function
  URL plumbing.
- Deploys are slower (version publish + distribution update), but the
  architecture is simple: CloudFront → S3 bucket (OAC), Lambda@Edge
  intercepts origin-request events and replaces the response.

**Constraints to keep in mind (see also Gotchas below):**

- Lambda must be deployed in `us-east-1` (Lambda@Edge requirement).
- Lambda@Edge cannot have environment variables. Bucket name must be
  baked in at build time.
- Use `EventType: origin-request` (not `origin-response`).
  `IncludeBody: true` is only valid on viewer-request or
  origin-request; origin-response + `IncludeBody` is rejected by AWS.
- Each Lambda code change requires `publish-version` and a distribution
  update pointing to the new versioned ARN.

The canonical reference implementation is the old project's
Lambda@Edge handler:
`/Volumes/Pascal4Tb/Projects/claude/astrophoto/main/lambda-deploy/image-transformer.js`

The staging deployment artefact (NOT committed to this repo) is at
`/tmp/astrophoto-staging-lambda/index.cjs` on the provisioning
machine.

---

## Environments

| env     | bucket                    | region    | CDN host                                       |
|---------|---------------------------|-----------|------------------------------------------------|
| dev     | astrophoto-images-dev     | ap-southeast-1 | `http://localhost:8080/cdn/img` (backend route) |
| staging | astrophoto-images-staging | us-east-1 | `https://ddo5booq71gbx.cloudfront.net`         |
| prod    | astrophoto-images-prod    | us-east-1 | `https://cdn.astrophoto.pics`                  |

**Why `us-east-1` for prod/staging?** Lambda@Edge functions must be
deployed in `us-east-1`. Placing the S3 bucket in the same region
avoids a cross-region round-trip on every origin-request cache miss.
`us-east-1` also carries the lowest S3 data transfer pricing to
CloudFront.

**Dev asymmetry is intentional.** The `astrophoto-images-dev` bucket
was provisioned earlier in `ap-southeast-1` and is the proven IAM /
bucket-policy template for this runbook. The CDN layer is not deployed
in dev; the backend exposes `GET /cdn/img/<photo-id>?w=&h=...` which
performs equivalent transforms using the Rust `image` crate. Staging
can bypass CloudFront entirely by setting `APP_CDN_LOCAL_FALLBACK=true`
(see §3.1).

---

## 1. S3 bucket

### 1.1 Create

```bash
REGION=us-east-1
BUCKET=astrophoto-images-prod

# us-east-1 is the S3 default region; omit LocationConstraint for it.
aws s3api create-bucket \
  --bucket "$BUCKET" \
  --region "$REGION"
```

For any other region, add:
```bash
  --create-bucket-configuration LocationConstraint="$REGION"
```

Verify:
```bash
aws s3api head-bucket --bucket "$BUCKET"
```

### 1.2 Block all public access

CloudFront reads the bucket via S3 OAC. No object should ever be
directly public. Enforce this at the bucket level so it cannot be
overridden by a future ACL or policy mistake:

```bash
aws s3api put-public-access-block \
  --bucket "$BUCKET" \
  --public-access-block-configuration \
    BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true
```

Verify:
```bash
aws s3api get-public-access-block --bucket "$BUCKET"
```

All four fields must be `true`.

### 1.3 Bucket ownership controls

Disable ACLs. Bucket owner enforced means `PutObjectAcl` is not
needed and cannot be used to make objects public:

```bash
aws s3api put-bucket-ownership-controls \
  --bucket "$BUCKET" \
  --ownership-controls 'Rules=[{ObjectOwnership=BucketOwnerEnforced}]'
```

### 1.4 Lifecycle: abort stale multipart uploads

Write `lifecycle.json`:

```json
{
  "Rules": [
    {
      "ID": "abort-stale-multiparts",
      "Status": "Enabled",
      "Filter": { "Prefix": "" },
      "AbortIncompleteMultipartUpload": { "DaysAfterInitiation": 1 }
    }
  ]
}
```

Apply:

```bash
aws s3api put-bucket-lifecycle-configuration \
  --bucket "$BUCKET" \
  --lifecycle-configuration file://lifecycle.json
```

### 1.5 CORS

The browser PUTs directly to S3 via presigned URL. CORS must allow
the production origin to PUT and read the `ETag` response header
(used for dedup confirmation).

Write `cors.json`:

```json
{
  "CORSRules": [
    {
      "AllowedHeaders": ["*"],
      "AllowedMethods": ["PUT", "GET", "HEAD"],
      "AllowedOrigins": [
        "https://astrophoto.pics",
        "https://www.astrophoto.pics"
      ],
      "ExposeHeaders": ["ETag"],
      "MaxAgeSeconds": 3000
    }
  ]
}
```

Apply:

```bash
aws s3api put-bucket-cors \
  --bucket "$BUCKET" \
  --cors-configuration file://cors.json
```

### 1.6 IAM user for backend (`astrophoto-prod-uploader`)

The backend needs to write originals and display masters, read objects
for HEAD verification, and delete objects when a photo is removed.

**Create the user:**

```bash
aws iam create-user --user-name astrophoto-prod-uploader
```

**Write the inline policy** (`uploader-policy.json`):

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "ObjectOperations",
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:GetObject",
        "s3:DeleteObject",
        "s3:HeadObject"
      ],
      "Resource": "arn:aws:s3:::astrophoto-images-prod/*"
    },
    {
      "Sid": "BucketList",
      "Effect": "Allow",
      "Action": "s3:ListBucket",
      "Resource": "arn:aws:s3:::astrophoto-images-prod"
    }
  ]
}
```

**Attach the policy:**

```bash
aws iam put-user-policy \
  --user-name astrophoto-prod-uploader \
  --policy-name astrophoto-prod-uploader-policy \
  --policy-document file://uploader-policy.json
```

**Generate access key:**

```bash
aws iam create-access-key --user-name astrophoto-prod-uploader
```

Store the `AccessKeyId` and `SecretAccessKey` in your secrets manager
(AWS Secrets Manager, Doppler, or the prod environment's secret
store). Do not log them.

---

## 2. CloudFront + Lambda@Edge

### 2.1 Lambda@Edge handler

The handler is a CommonJS module (Lambda@Edge runs Node 20; the `.cjs`
extension is explicit to avoid ESM parsing issues when bundled with
CommonJS `require`s).

**URL contract:** CloudFront routes `/img/<photo-id>` to the
distribution. The Lambda@Edge function receives the `origin-request`
event, matches the URI pattern, fetches `display/<photo-id>.jpg` from
S3, transforms it, and returns a synthetic response. Any URI that does
not match `/img/<photo-id>` is returned as-is so CloudFront proxies it
to S3 (or returns 403/404 per the bucket policy).

```javascript
// index.cjs — Lambda@Edge origin-request handler
// Node 20 + sharp + @aws-sdk/client-s3 (CommonJS, no env vars)
'use strict';

const sharp = require('sharp');
const querystring = require('querystring');
const { S3Client, GetObjectCommand } = require('@aws-sdk/client-s3');

// Lambda@Edge has no env vars — bucket name is baked in at build time.
// Separate Lambda functions are required for staging vs prod.
const BUCKET = 'astrophoto-images-prod';   // change to -staging for staging build
const REGION = 'us-east-1';
const s3 = new S3Client({ region: REGION });

exports.handler = async (event) => {
  const { request } = event.Records[0].cf;

  if (request.method !== 'GET' && request.method !== 'HEAD') {
    return request;
  }

  // Match /img/<photo-id>; pass through anything else.
  const m = request.uri.match(/^\/img\/([0-9a-f-]+)$/i);
  if (!m) return request;
  const photoId = m[1];
  const key = `display/${photoId}.jpg`;

  const params = querystring.parse(request.querystring);
  const w = params.w ? Math.min(parseInt(params.w, 10), 4096) : undefined;
  const h = params.h ? Math.min(parseInt(params.h, 10), 4096) : undefined;
  const q = params.q ? Math.min(Math.max(parseInt(params.q, 10), 1), 100) : 85;
  const fit = ['cover', 'contain', 'fill', 'inside', 'outside'].includes(params.fit)
    ? params.fit : 'inside';
  const fm = (params.fm || 'jpeg').toLowerCase();

  try {
    const out = await s3.send(new GetObjectCommand({ Bucket: BUCKET, Key: key }));
    const chunks = [];
    for await (const c of out.Body) chunks.push(c);
    const inputBuf = Buffer.concat(chunks);

    let pipeline = sharp(inputBuf, { failOnError: false });
    if (w || h) {
      pipeline = pipeline.resize({ width: w, height: h, fit, withoutEnlargement: true });
    }

    let outputBuf, contentType;
    if (fm === 'webp') {
      outputBuf = await pipeline.webp({ quality: q }).toBuffer();
      contentType = 'image/webp';
    } else if (fm === 'avif') {
      outputBuf = await pipeline.avif({ quality: q }).toBuffer();
      contentType = 'image/avif';
    } else if (fm === 'png') {
      outputBuf = await pipeline.png({ quality: q }).toBuffer();
      contentType = 'image/png';
    } else {
      outputBuf = await pipeline.jpeg({ quality: q, mozjpeg: true }).toBuffer();
      contentType = 'image/jpeg';
    }

    return {
      status: '200',
      statusDescription: 'OK',
      headers: {
        'content-type':  [{ key: 'Content-Type',  value: contentType }],
        'cache-control': [{ key: 'Cache-Control',
                            value: 'public, max-age=31536000, immutable' }],
      },
      body: outputBuf.toString('base64'),
      bodyEncoding: 'base64',
    };
  } catch (e) {
    // On any error, fall through to origin (CloudFront serves S3 or 404).
    console.error('image-transformer error:', e && e.message);
    return request;
  }
};
```

**Key differences from the Function URL handler** (commit `1837d0a`):
- Event shape is `event.Records[0].cf.request` (Lambda@Edge), not
  `event.rawPath` (Function URL).
- Response shape uses `status` (string), `headers` (array-of-objects),
  `bodyEncoding: 'base64'` (Lambda@Edge), not `statusCode` (number),
  `headers` (plain object), `isBase64Encoded` (Function URL).
- No env vars; `BUCKET` is a constant.

**`fm` is required from the frontend.** The cache policy
(`HeaderBehavior: none`) does not include the `Accept` header in the
cache key. If the handler fell back to `Accept`-based format
negotiation when `fm` is absent, the first browser to request a URL
without `fm` would write its preferred format into CloudFront's cache
for all subsequent browsers — a correctness bug. The frontend
`<Img>` component must always emit an explicit `fm` value. The handler
defaults to `jpeg` when `fm` is absent (safe for the cache key
because the default is deterministic).

**Lambda@Edge response size limit:** the generated response body must
be ≤ 1 MB for `origin-request` triggers. Display masters at
4096 px / q=85 are typically under 1 MB but large TIFF-sourced masters
can approach it. The `withoutEnlargement: true` option prevents
upscaling. Monitor CloudWatch logs on staging after uploading large
images to confirm.

### 2.2 Packaging sharp for Linux x86_64

`sharp` requires native binaries compiled for the Lambda runtime. Build
on a Linux x86_64 host or use the npm platform flags:

```bash
# From the Lambda source directory
npm install --cpu=x64 --os=linux --include=optional

# zip the handler + node_modules
zip -r function.zip index.cjs node_modules/
```

If building on macOS or Windows, use a Docker build matching the
Lambda runtime:

```bash
docker run --rm \
  -v "$PWD":/var/task \
  -w /var/task \
  public.ecr.aws/lambda/nodejs:20 \
  npm install --cpu=x64 --os=linux --include=optional

zip -r function.zip index.cjs node_modules/
```

The staging deployment at `/tmp/astrophoto-staging-lambda/` was built
with `npm install --cpu=x64 --os=linux --include=optional` on macOS
and the binary worked correctly in Lambda@Edge (Node 20, x86_64).

### 2.3 Lambda execution role

The execution role must include BOTH `lambda.amazonaws.com` AND
`edgelambda.amazonaws.com` as trusted principals. Without the
`edgelambda.amazonaws.com` principal, CloudFront cannot replicate the
function to edge locations.

**Create the role:**

```bash
aws iam create-role \
  --role-name astrophoto-staging-lambda-exec \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [
      {
        "Effect": "Allow",
        "Principal": {
          "Service": [
            "lambda.amazonaws.com",
            "edgelambda.amazonaws.com"
          ]
        },
        "Action": "sts:AssumeRole"
      }
    ]
  }' \
  --region us-east-1
```

Attach the AWS-managed basic execution policy (CloudWatch Logs):

```bash
aws iam attach-role-policy \
  --role-name astrophoto-staging-lambda-exec \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
```

The Lambda function reads S3 via execution role. Grant `s3:GetObject`
on `display/*`:

```bash
aws iam put-role-policy \
  --role-name astrophoto-staging-lambda-exec \
  --policy-name s3-display-read \
  --policy-document '{
    "Version": "2012-10-17",
    "Statement": [{
      "Effect": "Allow",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::astrophoto-images-prod/display/*"
    }]
  }'
```

Use `astrophoto-prod-lambda-exec` and `astrophoto-images-prod` for
prod.

### 2.4 Deploy the Lambda function

**Create the function** (after building `function.zip` per §2.2):

```bash
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
ROLE_ARN="arn:aws:iam::${ACCOUNT_ID}:role/astrophoto-staging-lambda-exec"

aws lambda create-function \
  --function-name astrophoto-staging-image-transformer \
  --runtime nodejs20.x \
  --role "$ROLE_ARN" \
  --handler index.handler \
  --zip-file fileb://function.zip \
  --timeout 10 \
  --memory-size 1024 \
  --region us-east-1
```

**No `--environment` flag.** Lambda@Edge does not support environment
variables. The bucket name must be baked into the handler code (see
§2.1).

**Update the function** (subsequent deploys):

```bash
aws lambda update-function-code \
  --function-name astrophoto-staging-image-transformer \
  --zip-file fileb://function.zip \
  --region us-east-1
```

### 2.5 Publish a version (required for Lambda@Edge)

Lambda@Edge requires a versioned ARN — the `$LATEST` alias is not
accepted. After every code change, publish a new version:

```bash
aws lambda publish-version \
  --function-name astrophoto-staging-image-transformer \
  --region us-east-1
```

Note the `Version` number in the output. The versioned ARN is:

```
arn:aws:lambda:us-east-1:<ACCOUNT_ID>:function:astrophoto-staging-image-transformer:<VERSION>
```

This ARN is what goes into the CloudFront distribution config.
Updating the distribution to point at the new versioned ARN after each
republish is mandatory.

### 2.6 CloudFront distribution

**Step 1 — Create an S3 OAC**

This OAC grants CloudFront signed read access to the S3 bucket. Use
`OriginAccessControlOriginType: "s3"` (not `"lambda"` — that was for
the Function URL pattern in the old runbook).

```bash
aws cloudfront create-origin-access-control \
  --origin-access-control-config '{
    "Name": "astrophoto-s3-oac",
    "Description": "Signs CF requests to the S3 bucket via OAC",
    "SigningProtocol": "sigv4",
    "SigningBehavior": "always",
    "OriginAccessControlOriginType": "s3"
  }'
```

Note the `Id` in the output (`EXZAEKMIK0D15` in staging). You need
it in the distribution config.

**Step 2 — Create the cache policy**

```bash
aws cloudfront create-cache-policy \
  --cache-policy-config '{
    "Name": "astrophoto-image-transformer-cache",
    "Comment": "Includes w, h, fit, q, fm in cache key; no headers/cookies",
    "DefaultTTL": 86400,
    "MaxTTL": 31536000,
    "MinTTL": 1,
    "ParametersInCacheKeyAndForwardedToOrigin": {
      "EnableAcceptEncodingGzip": true,
      "EnableAcceptEncodingBrotli": true,
      "HeadersConfig": { "HeaderBehavior": "none" },
      "CookiesConfig": { "CookieBehavior": "none" },
      "QueryStringsConfig": {
        "QueryStringBehavior": "whitelist",
        "QueryStrings": {
          "Quantity": 5,
          "Items": ["w", "h", "fit", "q", "fm"]
        }
      }
    }
  }'
```

Note the `Id` in the output (`c80acb91-dc64-4019-ada6-09ff3d2098be`
in staging). Use it as `CACHE_POLICY_ID` below.

The cache policy includes `HeaderBehavior: none` — the `Accept`
header is intentionally excluded from the cache key. Format selection
is driven entirely by the `fm` query param (see §2.1 on cache
correctness). **Do not use** `Managed-CachingOptimized` — it strips
query strings from the cache key.

**Step 3 — Create the distribution**

Write `dist-config.json` (replace placeholder values; `ACCOUNT_ID`,
`VERSIONED_LAMBDA_ARN`, `S3_OAC_ID`, `CACHE_POLICY_ID`):

**`CallerReference` must be unique per attempt.** It is an idempotency
key — change it if you're re-running after a failed create.

```json
{
  "CallerReference": "astrophoto-prod-cf-1",
  "Comment": "Astrophoto prod image CDN",
  "Enabled": true,
  "HttpVersion": "http2and3",
  "IsIPV6Enabled": true,
  "PriceClass": "PriceClass_100",
  "Origins": {
    "Quantity": 1,
    "Items": [
      {
        "Id": "s3-display-origin",
        "DomainName": "astrophoto-images-prod.s3.us-east-1.amazonaws.com",
        "S3OriginConfig": { "OriginAccessIdentity": "" },
        "OriginAccessControlId": "S3_OAC_ID"
      }
    ]
  },
  "DefaultCacheBehavior": {
    "TargetOriginId": "s3-display-origin",
    "ViewerProtocolPolicy": "redirect-to-https",
    "AllowedMethods": {
      "Quantity": 2,
      "Items": ["GET", "HEAD"],
      "CachedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
    },
    "Compress": true,
    "CachePolicyId": "CACHE_POLICY_ID",
    "LambdaFunctionAssociations": {
      "Quantity": 1,
      "Items": [
        {
          "LambdaFunctionARN": "VERSIONED_LAMBDA_ARN",
          "EventType": "origin-request",
          "IncludeBody": false
        }
      ]
    }
  }
}
```

Notes on the config:
- `DomainName`: the S3 bucket's regional endpoint. Use the full
  regional form `<bucket>.s3.<region>.amazonaws.com`.
- `S3OriginConfig: { "OriginAccessIdentity": "" }`: the empty string
  is required when using OAC (not OAI). Setting it to empty opts into
  the newer OAC path; `OriginAccessControlId` on the origin item
  specifies the actual OAC.
- `LambdaFunctionARN`: must be the versioned ARN (e.g.,
  `arn:aws:lambda:us-east-1:123456:function:name:1`). `$LATEST` is
  rejected by CloudFront.
- `EventType: "origin-request"`: this is the correct event type for a
  transform-and-replace handler. Do NOT use `origin-response` — AWS
  now rejects distributions with `origin-response` + `IncludeBody: true`.
- The distribution does NOT need a custom domain for staging. For prod,
  add `Aliases`, `ViewerCertificate` (ACM cert in `us-east-1`), and
  a DNS CNAME.

```bash
aws cloudfront create-distribution \
  --distribution-config file://dist-config.json
```

Note the `Id` (`E2B1QQ4K2EISGE` in staging) and `DomainName`
(`ddo5booq71gbx.cloudfront.net` in staging).

### 2.7 S3 bucket policy: grant CloudFront OAC read on `display/*`

The bucket policy grants `s3:GetObject` to `cloudfront.amazonaws.com`
conditioned on the OAC's source ARN (the distribution ARN). This is
the OAC pattern — do NOT grant the Lambda execution role directly in
the bucket policy; the Lambda reads S3 via its own IAM role.

Replace `ACCOUNT_ID`, `DISTRIBUTION_ID`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowCloudFrontOACReadDisplay",
      "Effect": "Allow",
      "Principal": {
        "Service": "cloudfront.amazonaws.com"
      },
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::astrophoto-images-prod/display/*",
      "Condition": {
        "StringEquals": {
          "aws:SourceArn": "arn:aws:cloudfront::ACCOUNT_ID:distribution/DISTRIBUTION_ID"
        }
      }
    }
  ]
}
```

Apply:

```bash
aws s3api put-bucket-policy \
  --bucket astrophoto-images-prod \
  --policy file://bucket-policy.json
```

**Note:** The Lambda function reads S3 via its own execution role
(`astrophoto-staging-lambda-exec` / `astrophoto-prod-lambda-exec`),
not through CloudFront OAC. The role policy in §2.3 covers that path.
The bucket policy above covers CloudFront proxying non-`/img/*` paths
through to S3.

### 2.8 ACM certificate (prod only)

For a custom CDN hostname (`cdn.astrophoto.pics`), an ACM certificate
in `us-east-1` is required. Staging uses the raw CloudFront domain
(`*.cloudfront.net`) and does not need a certificate.

```bash
aws acm request-certificate \
  --domain-name cdn.astrophoto.pics \
  --subject-alternative-names "*.astrophoto.pics" \
  --validation-method DNS \
  --region us-east-1
```

Complete DNS validation and wait for `ISSUED` status before creating
the distribution.

Add to `dist-config.json` for prod:

```json
"Aliases": { "Quantity": 1, "Items": ["cdn.astrophoto.pics"] },
"ViewerCertificate": {
  "ACMCertificateArn": "arn:aws:acm:us-east-1:ACCOUNT_ID:certificate/CERT_ID",
  "SSLSupportMethod": "sni-only",
  "MinimumProtocolVersion": "TLSv1.2_2021"
}
```

And add a DNS CNAME:
```
cdn.astrophoto.pics  CNAME  <CloudFront domain>.cloudfront.net
```

### 2.9 Updating the Lambda function

Every code change requires republishing a version and updating the
distribution:

```bash
# 1. Push the updated zip
aws lambda update-function-code \
  --function-name astrophoto-staging-image-transformer \
  --zip-file fileb://function.zip \
  --region us-east-1

# 2. Publish a new version
aws lambda publish-version \
  --function-name astrophoto-staging-image-transformer \
  --region us-east-1
# Note the new Version number.

# 3. Get the current distribution ETag (needed for update)
aws cloudfront get-distribution-config \
  --id E2B1QQ4K2EISGE \
  --query '[ETag, DistributionConfig]' \
  --output json > current-dist.json

# 4. Edit the DistributionConfig JSON: update LambdaFunctionARN to the
#    new versioned ARN.

# 5. Apply the update
aws cloudfront update-distribution \
  --id E2B1QQ4K2EISGE \
  --if-match <ETAG_FROM_STEP_3> \
  --distribution-config file://updated-dist-config.json
```

Distribution updates take ~5 min to propagate globally.

---

## 3. Production environment wiring

### 3.1 Backend env vars

Set these in the prod environment (`.env`, Kubernetes secret, Doppler,
or equivalent). No `APP_S3_ENDPOINT` — that variable is only set in
dev to point to MinIO. When unset, the `aws-sdk-s3` crate resolves
the standard AWS endpoint automatically.

```
APP_S3_REGION=us-east-1
APP_S3_BUCKET=astrophoto-images-prod
APP_S3_ACCESS_KEY=<AccessKeyId from astrophoto-prod-uploader>
APP_S3_SECRET_KEY=<SecretAccessKey from astrophoto-prod-uploader>
APP_S3_PATH_STYLE=false

# Do NOT set APP_S3_ENDPOINT — leave it unset for AWS endpoint resolution.

APP_CDN_BASE_URL=https://cdn.astrophoto.pics

# Optional — forces the backend's /cdn/img/<id> fallback route to activate
# on non-localhost hosts. Use in staging when CloudFront isn't provisioned
# yet, or in prod for emergency CDN bypass without a redeploy.
# APP_CDN_LOCAL_FALLBACK=true
```

`APP_S3_PATH_STYLE=false` enables virtual-hosted-style URLs
(`bucket.s3.amazonaws.com`) required by AWS S3. In dev, MinIO needs
`APP_S3_PATH_STYLE=true` because it uses path-style by default.

`APP_CDN_LOCAL_FALLBACK=true` opts into the backend's `/cdn/img/`
route even on non-localhost hosts. This is the staging bypass used
before CloudFront was provisioned. Once CloudFront is live, leave this
unset.

### 3.2 Frontend env vars

```
PUBLIC_CDN_BASE_URL=https://cdn.astrophoto.pics
VITE_API_BASE_URL=https://astrophoto.pics
```

`PUBLIC_CDN_BASE_URL` is the value read by the frontend `<Img>`
component (and the CDN URL builder) to construct image URLs. In dev
this is set to `http://localhost:8080/cdn` (the backend local route).

### 3.3 R2 → S3 migration (one-shot, conditional)

This section applies only if an environment was previously running
with Cloudflare R2. For a new prod deployment starting from Phase 1,
skip this section.

**If migrating from an existing R2 deployment:**

```bash
# Prerequisites: rclone configured with two remotes:
#   r2-source  — Cloudflare R2 (S3-compatible endpoint)
#   s3-prod    — AWS S3 (standard)

rclone copy r2-source:astrophoto-r2/originals/ \
  s3-prod:astrophoto-images-prod/originals/ \
  --progress --transfers 8

rclone copy r2-source:astrophoto-r2/display/ \
  s3-prod:astrophoto-images-prod/display/ \
  --progress --transfers 8
```

**Decommission R2 only after:**
1. CloudFront is serving real traffic from S3.
2. All photo URLs in the database resolve correctly via the new CDN.
3. A full backup of R2 originals is confirmed on S3.

---

## 4. Gotchas

- **`origin-response` + `IncludeBody: true` is rejected.** AWS now
  rejects CloudFront distributions that set `EventType: origin-response`
  with `IncludeBody: true` on update. Transform-and-replace handlers
  must use `EventType: origin-request`. The old project's config used
  origin-response; this project uses origin-request.

- **No env vars on Lambda@Edge.** The `--environment` flag is silently
  ignored (or rejected) for Lambda@Edge. The bucket name must be baked
  into the handler at build time. Staging and prod require separate
  Lambda functions (`astrophoto-staging-image-transformer`,
  `astrophoto-prod-image-transformer`) with different hardcoded bucket
  names.

- **Lambda must be in `us-east-1`.** Lambda@Edge is only supported for
  functions deployed in `us-east-1`. Functions in other regions cannot
  be attached to CloudFront distributions.

- **Trust policy must include `edgelambda.amazonaws.com`.** Without
  this principal, CloudFront cannot assume the role to replicate the
  function. The role creation in §2.3 includes both `lambda.amazonaws.com`
  and `edgelambda.amazonaws.com`.

- **Every code change requires `publish-version`.** Lambda@Edge uses
  versioned ARNs. `$LATEST` is not accepted. After each code update:
  `update-function-code` → `publish-version` → update distribution
  with the new versioned ARN → wait ~5 min for propagation.

- **FUBPA blocks Function URLs.** Lambda Function URL Block Public
  Access (FUBPA) is active in this AWS account. Function URLs with
  `AuthType: NONE` are silently inaccessible. The deleted function URL
  (`vmt44kiqpygriigehgzorzflyy0wbpgp.lambda-url.us-east-1.on.aws`) is
  documented here as a historical reference — do not recreate it.

- **S3 OAC vs Lambda OAC.** The S3 OAC uses `OriginAccessControlOriginType: "s3"`.
  The old runbook (commit `1837d0a`) used `OriginAccessControlOriginType: "lambda"`
  for a Lambda Function URL origin. These are different; using the wrong type
  causes CloudFront to use the wrong signing method.

- **Display master response size.** Lambda@Edge origin-request
  generated responses are capped at 1 MB. Display masters at 4096 px
  / q=85 are typically under 1 MB but can approach it for very
  detailed images. If the Lambda returns the `request` object on error
  (fall-through path), CloudFront will fetch from S3 directly and the
  client sees the untransformed display master.

---

## 5. Verification checklist

- [ ] Bucket exists in `us-east-1`.
- [ ] All four Block-Public-Access flags are `true`.
- [ ] Bucket ownership set to `BucketOwnerEnforced` (ACLs disabled).
- [ ] CORS allows the prod origin for `PUT`, `GET`, `HEAD`.
- [ ] Lifecycle rule aborts incomplete multipart uploads after 1 day.
- [ ] Bucket policy grants `s3:GetObject` on `display/*` to
      `cloudfront.amazonaws.com` with `aws:SourceArn` condition
      matching the distribution ARN.
- [ ] IAM user `astrophoto-prod-uploader` exists with scoped policy.
- [ ] Lambda function `astrophoto-prod-image-transformer` deployed in
      `us-east-1`, runtime `nodejs20.x`.
- [ ] Lambda function has **no** `--environment` variables.
- [ ] Lambda execution role trust policy includes BOTH
      `lambda.amazonaws.com` AND `edgelambda.amazonaws.com`.
- [ ] Lambda execution role has `s3:GetObject` on
      `astrophoto-images-prod/display/*`.
- [ ] Lambda version published (`publish-version`); versioned ARN
      noted.
- [ ] CloudFront OAC created with `OriginAccessControlOriginType: "s3"`.
- [ ] Distribution origin is the S3 bucket regional endpoint (not a
      Lambda function URL).
- [ ] `DefaultCacheBehavior.LambdaFunctionAssociations` uses
      `EventType: "origin-request"` with the versioned Lambda ARN.
- [ ] Custom cache policy whitelists `w`, `h`, `fit`, `q`, `fm`.
- [ ] (Prod only) ACM certificate in `us-east-1` status `ISSUED`.
- [ ] (Prod only) Distribution `Aliases` includes `cdn.astrophoto.pics`.
- [ ] (Prod only) DNS CNAME `cdn.astrophoto.pics` → CloudFront domain.
- [ ] `curl -s -o /dev/null -w "%{http_code}" "https://<CF_DOMAIN>/img/<id>?w=400&fm=jpeg"` returns `200`.
- [ ] Second identical request returns `x-cache: Hit from cloudfront`.
- [ ] Different `w` values are cached separately (cache policy test).
- [ ] Backend env: `APP_S3_ENDPOINT` is **unset**.
- [ ] Backend env: `APP_S3_PATH_STYLE=false`.
- [ ] Backend env: `APP_CDN_BASE_URL` points at the CloudFront domain.
- [ ] Frontend env: `PUBLIC_CDN_BASE_URL` points at the CloudFront domain.

---

## 6. References

- Spec: `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
- Plan: `docs/superpowers/plans/2026-05-03-photographer-showcase-p1-foundations.md`
- Old project canonical Lambda@Edge source:
  `/Volumes/Pascal4Tb/Projects/claude/astrophoto/main/lambda-deploy/image-transformer.js`
- Staging Lambda artefact (not committed; on provisioning machine):
  `/tmp/astrophoto-staging-lambda/index.cjs`
- AWS docs — Lambda@Edge:
  https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/lambda-at-the-edge.html
- AWS docs — CloudFront OAC for S3:
  https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/private-content-restricting-access-to-s3.html

### Staged deployment reference (staging, 2026-05)

Actual resource IDs from the validated staging deployment:

| resource                 | ID / ARN                                                          |
|--------------------------|-------------------------------------------------------------------|
| S3 bucket                | `astrophoto-images-staging` (us-east-1)                          |
| IAM user                 | `astrophoto-staging-uploader`                                    |
| Lambda role              | `astrophoto-staging-lambda-exec`                                  |
| Lambda function          | `astrophoto-staging-image-transformer:1`                         |
| CloudFront distribution  | `E2B1QQ4K2EISGE` → `ddo5booq71gbx.cloudfront.net`               |
| CloudFront S3 OAC        | `EXZAEKMIK0D15`                                                  |
| Cache policy             | `c80acb91-dc64-4019-ada6-09ff3d2098be`                          |
| Koyeb backend            | `https://astrophoto-staging-xavyo-008151d0.koyeb.app`           |
| Koyeb frontend           | `https://astrophoto-staging-web-xavyo-eadbe1f6.koyeb.app`       |

The deleted Function URL
(`vmt44kiqpygriigehgzorzflyy0wbpgp.lambda-url.us-east-1.on.aws`) was
created and then deleted after the architecture pivot to Lambda@Edge.
Do not recreate it.
