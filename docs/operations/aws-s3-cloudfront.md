# AWS S3 + CloudFront — Astrophoto images

Runbook for provisioning the photo-storage and CDN infrastructure for
the Astrophoto Phase 1 (Photographer Showcase) deployment.

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
fetches the small JPEG from the Lambda, not the raw original. This
decouples the CDN resizer from heavy source formats (16-bit TIFF at
200 MB), cuts Lambda memory / cold-start cost, and keeps transform
logic simple.

**Traffic path:**

```
Browser
  └─► CloudFront (cdn.astrophoto.pics)
        └─► Lambda function URL (Node 20 + sharp)
              └─► S3  display/<photo-id>.jpg
```

CloudFront caches responses keyed on the full URL including query
string (`w`, `h`, `fit`, `q`, `fm`). Lambda fetches the display
master from S3 on a cache miss, transforms it, and returns the result.
The backend (`astrophoto-prod-uploader` IAM user) writes both
`originals/` and `display/` directly via `PutObject`. Browsers never
reach S3 — all read traffic goes through CloudFront.

**This is NOT the Lambda@Edge pattern** from the previous project. The
old project used Lambda@Edge as an origin-response trigger: CloudFront
fetched from S3 first, then Lambda mutated the body. The new pattern
uses a Lambda function URL as the CloudFront origin: Lambda fetches
from S3 itself via its execution role, transforms, and returns. This
avoids Lambda@Edge constraints (max 10 MB response, 30 s timeout, no
native deps compiled for the Lambda machine, restricted regions) and
simplifies deploys to a standard regional Lambda.

---

## Environments

| env     | bucket                    | region         | CDN host                                     |
|---------|---------------------------|----------------|----------------------------------------------|
| dev     | astrophoto-images-dev     | ap-southeast-1 | `http://localhost:8080/cdn/img` (backend route) |
| staging | astrophoto-images-staging | us-east-1      | `https://cdn-staging.astrophoto.pics`        |
| prod    | astrophoto-images-prod    | us-east-1      | `https://cdn.astrophoto.pics`                |

**Why `us-east-1` for prod/staging?** ACM certificates attached to
CloudFront distributions must be provisioned in `us-east-1` regardless
of where CloudFront POPs are. Placing the S3 bucket in the same region
as the certificate removes a cross-region S3 round-trip from every
Lambda cold-path fetch. `us-east-1` also carries the lowest S3 data
transfer pricing to CloudFront.

**Dev asymmetry is intentional.** The `astrophoto-images-dev` bucket
was provisioned earlier in `ap-southeast-1` and is the proven IAM /
bucket-policy template for this runbook. The CDN layer is not deployed
in dev; the backend exposes `GET /cdn/img/<photo-id>?w=&h=...` which
performs equivalent transforms using the Rust `image` crate.

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

CloudFront reads the bucket through the Lambda execution role. No
object should ever be directly public. Enforce this at the bucket level
so it cannot be overridden by a future ACL or policy mistake:

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

Disable ACLs. Modern S3 best practice: bucket owner enforced means
`PutObjectAcl` is not needed and cannot be used to make objects public:

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

The browser PUTs directly to S3 via presigned URL. CORS must allow the
production origin to PUT and read the `ETag` response header (used for
dedup confirmation).

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

### 1.6 Bucket policy: grant Lambda execution role read on `display/*`

The Lambda function that backs the CloudFront origin reads
`display/<photo-id>.jpg` using its IAM execution role. The bucket
policy grants `s3:GetObject` on that prefix to the role ARN.

Replace `ACCOUNT_ID` and `DISTRIBUTION_ID` with real values.

Write `bucket-policy.json`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowLambdaExecutionRoleReadDisplay",
      "Effect": "Allow",
      "Principal": {
        "AWS": "arn:aws:iam::ACCOUNT_ID:role/astrophoto-image-transformer-role"
      },
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::astrophoto-images-prod/display/*"
    }
  ]
}
```

Apply:

```bash
aws s3api put-bucket-policy \
  --bucket "$BUCKET" \
  --policy file://bucket-policy.json
```

**Note:** This policy does NOT grant `s3:GetObject` on `originals/*`.
The Lambda only needs display masters. The backend IAM user
(`astrophoto-prod-uploader`) has `GetObject` on `/*` for its own
operations (HEAD, finalize verification) but browsers never hit S3
directly.

### 1.7 IAM user for backend (`astrophoto-prod-uploader`)

The backend needs to write originals and display masters, read objects
for HEAD verification, and delete objects when a photo is removed.
This mirrors the `astrophoto-dev-uploader` setup used for dev.

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
(AWS Secrets Manager, Doppler, or the prod environment's secret store).
Do not log them.

---

## 2. Lambda function URL (image transformer)

### 2.1 Why Lambda function URL, not Lambda@Edge

Lambda@Edge runs at CloudFront edge locations and has hard constraints
that make it unsuitable here:

- Max response payload: 1 MB for origin-response triggers (display
  masters can be several MB).
- Max timeout: 30 s (cold starts with native binaries can exceed this).
- Cannot use native binary dependencies compiled for the Lambda
  execution environment without careful bundling.
- Deploys are replicated to every edge region; rollbacks are slow.

A Lambda function URL is a regional HTTPS endpoint for a single Lambda
function. CloudFront uses it as a custom origin. Characteristics:

- Timeout: up to 15 minutes (10 s is ample for image transforms).
- Memory: up to 10 GB.
- Native `sharp` binary installed in the same region as the function —
  no cross-region complications.
- Standard Lambda deployment / versioning / rollback.
- Can set `AuthType: AWS_IAM` and restrict invocation to the
  CloudFront distribution via OAC (see §2.3).

### 2.2 Create the Lambda execution role

```bash
aws iam create-role \
  --role-name astrophoto-image-transformer-role \
  --assume-role-policy-document '{
    "Version": "2012-10-17",
    "Statement": [{
      "Effect": "Allow",
      "Principal": { "Service": "lambda.amazonaws.com" },
      "Action": "sts:AssumeRole"
    }]
  }'
```

Attach the AWS-managed basic execution policy (CloudWatch Logs):

```bash
aws iam attach-role-policy \
  --role-name astrophoto-image-transformer-role \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
```

Attach an inline policy for S3 read on `display/*`:

```bash
aws iam put-role-policy \
  --role-name astrophoto-image-transformer-role \
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

After creating this role, go back to §1.6 and fill in the role ARN in
the bucket policy, then re-apply `put-bucket-policy`.

### 2.3 Deploy the Lambda function

The production Lambda code lives in a separate repo or `infra/lambda/`
directory. For reference, the handler is a function-URL style handler
(NOT Lambda@Edge). The function-URL event shape and the ported
transform logic from the old project's `image-transformer.js` are
documented below (§2.4).

**Create the function** (after bundling `index.js` + `node_modules`
into `function.zip`):

```bash
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
ROLE_ARN="arn:aws:iam::${ACCOUNT_ID}:role/astrophoto-image-transformer-role"

aws lambda create-function \
  --function-name astrophoto-image-transformer \
  --runtime nodejs20.x \
  --role "$ROLE_ARN" \
  --handler index.handler \
  --zip-file fileb://function.zip \
  --timeout 10 \
  --memory-size 1024 \
  --environment "Variables={BUCKET=astrophoto-images-prod,REGION=us-east-1}" \
  --region us-east-1
```

**Create the function URL** with IAM auth (CloudFront OAC will sign
requests; this prevents direct invocation without CloudFront):

```bash
aws lambda create-function-url-config \
  --function-name astrophoto-image-transformer \
  --auth-type AWS_IAM \
  --region us-east-1
```

Note the `FunctionUrl` in the output (e.g.,
`https://abcdef1234567890.lambda-url.us-east-1.on.aws/`). You will
need it when configuring the CloudFront origin in §2.5.

**Update the function** (subsequent deploys):

```bash
aws lambda update-function-code \
  --function-name astrophoto-image-transformer \
  --zip-file fileb://function.zip \
  --region us-east-1
```

### 2.4 Handler pseudocode (function-URL shape)

The handler is adapted from the old project's Lambda@Edge
`image-transformer.js`. The transform logic (sharp params, format
negotiation via `Accept` header) is preserved; the event shape and S3
fetch are rewritten for the function-URL pattern.

```javascript
// index.js — Lambda function URL handler (Node 20 + sharp + @aws-sdk/client-s3)
import sharp from 'sharp';
import { S3Client, GetObjectCommand } from '@aws-sdk/client-s3';

const s3 = new S3Client({ region: process.env.REGION });
const BUCKET = process.env.BUCKET;
const MAX_DIM = 4096;

export const handler = async (event) => {
  // Path: /img/<photo-id>  (CloudFront strips the /img prefix if configured)
  const photoId = event.rawPath.replace(/^\/img\//, '').replace(/[^a-zA-Z0-9_-]/g, '');
  const q = event.queryStringParameters ?? {};
  const accept = (event.headers?.accept ?? '');

  // Fetch display master from S3
  let s3Body;
  try {
    const resp = await s3.send(new GetObjectCommand({
      Bucket: BUCKET,
      Key: `display/${photoId}.jpg`,
    }));
    s3Body = Buffer.from(await resp.Body.transformToByteArray());
  } catch (err) {
    return { statusCode: 404, body: 'not found' };
  }

  // Determine output format (fm param or Accept-header negotiation)
  const fmParam = (q.fm ?? '').toLowerCase();
  const fmt = ['jpeg','webp','avif'].includes(fmParam) ? fmParam
    : accept.includes('image/avif') ? 'avif'
    : accept.includes('image/webp') ? 'webp'
    : 'jpeg';

  const w = Math.min(parseInt(q.w, 10) || 0, MAX_DIM) || undefined;
  const h = Math.min(parseInt(q.h, 10) || 0, MAX_DIM) || undefined;
  const fit = ['cover','contain','fill','inside','outside'].includes(q.fit)
    ? q.fit : 'inside';
  const quality = Math.min(Math.max(parseInt(q.q, 10) || 85, 1), 100);

  const buf = await sharp(s3Body, { failOnError: false })
    .resize({ width: w, height: h, fit, withoutEnlargement: true,
              background: { r: 0, g: 0, b: 0, alpha: 1 } })
    .toFormat(fmt, { quality, mozjpeg: fmt === 'jpeg' })
    .toBuffer();

  const cacheMaxAge = (w ?? 0) <= 400 ? 31536000 : (w ?? 0) <= 1200 ? 2592000 : 604800;
  return {
    statusCode: 200,
    headers: {
      'Content-Type': `image/${fmt === 'jpeg' ? 'jpeg' : fmt}`,
      'Cache-Control': `public, max-age=${cacheMaxAge}, immutable`,
      'Vary': 'Accept',
    },
    body: buf.toString('base64'),
    isBase64Encoded: true,
  };
};
```

**Packaging note:** `sharp` requires native binaries. Build inside a
Lambda-compatible environment:

```bash
# From your lambda source directory
npm ci --omit=dev --platform=linux --arch=x64 --libc=glibc
zip -r function.zip index.js node_modules/
```

Or use a Docker build matching the Lambda runtime:

```bash
docker run --rm -v "$PWD":/var/task \
  public.ecr.aws/lambda/nodejs:20 \
  npm ci --omit=dev
zip -r function.zip index.js node_modules/
```

### 2.5 ACM certificate

CloudFront requires an ACM certificate in `us-east-1` for custom
domains, regardless of where other resources are.

```bash
# Request a certificate covering both apex and www
aws acm request-certificate \
  --domain-name cdn.astrophoto.pics \
  --subject-alternative-names "*.astrophoto.pics" \
  --validation-method DNS \
  --region us-east-1
```

Complete DNS validation by adding the CNAME records shown in the ACM
console to your DNS provider. Wait until the certificate status is
`ISSUED` before creating the CloudFront distribution.

```bash
# Check status
aws acm describe-certificate \
  --certificate-arn arn:aws:acm:us-east-1:ACCOUNT_ID:certificate/CERT_ID \
  --region us-east-1 \
  --query 'Certificate.Status'
```

### 2.6 CloudFront distribution

**Step 1 — Create an OAC for CF→Lambda signing**

This OAC signs CloudFront requests to the Lambda function URL with
SigV4. Combined with `AuthType: AWS_IAM` on the function URL, it
prevents anyone from invoking the Lambda directly without going through
CloudFront.

```bash
aws cloudfront create-origin-access-control \
  --origin-access-control-config '{
    "Name": "astrophoto-lambda-oac",
    "Description": "Signs CF requests to the image transformer Lambda function URL",
    "SigningProtocol": "sigv4",
    "SigningBehavior": "always",
    "OriginAccessControlOriginType": "lambda"
  }'
```

Note the `Id` in the output (e.g., `E3ABCDEF12345`). You need it
below.

**Step 2 — Grant CloudFront permission to invoke the function**

After you know the distribution ARN (created in the next step), run:

```bash
aws lambda add-permission \
  --function-name astrophoto-image-transformer \
  --statement-id allow-cloudfront-invoke \
  --action lambda:InvokeFunctionUrl \
  --principal cloudfront.amazonaws.com \
  --source-arn "arn:aws:cloudfront::ACCOUNT_ID:distribution/DISTRIBUTION_ID" \
  --function-url-auth-type AWS_IAM \
  --region us-east-1
```

If you don't yet have the distribution ARN, create the distribution
first (Step 3), then come back and run this command.

**Step 3 — Create the distribution**

Write `dist-config.json` (replace placeholder values):

```json
{
  "CallerReference": "astrophoto-prod-cf-1",
  "Comment": "Astrophoto prod image CDN",
  "Enabled": true,
  "HttpVersion": "http2and3",
  "IsIPV6Enabled": true,
  "PriceClass": "PriceClass_100",
  "Aliases": {
    "Quantity": 1,
    "Items": ["cdn.astrophoto.pics"]
  },
  "ViewerCertificate": {
    "ACMCertificateArn": "arn:aws:acm:us-east-1:ACCOUNT_ID:certificate/CERT_ID",
    "SSLSupportMethod": "sni-only",
    "MinimumProtocolVersion": "TLSv1.2_2021"
  },
  "Origins": {
    "Quantity": 1,
    "Items": [
      {
        "Id": "lambda-image-transformer",
        "DomainName": "FUNCTION_URL_DOMAIN",
        "CustomOriginConfig": {
          "HTTPSPort": 443,
          "OriginProtocolPolicy": "https-only",
          "OriginSSLProtocols": { "Quantity": 1, "Items": ["TLSv1.2"] }
        },
        "OriginAccessControlId": "LAMBDA_OAC_ID"
      }
    ]
  },
  "DefaultCacheBehavior": {
    "TargetOriginId": "lambda-image-transformer",
    "ViewerProtocolPolicy": "redirect-to-https",
    "AllowedMethods": {
      "Quantity": 2,
      "Items": ["GET", "HEAD"],
      "CachedMethods": { "Quantity": 2, "Items": ["GET", "HEAD"] }
    },
    "Compress": true,
    "CachePolicyId": "CUSTOM_CACHE_POLICY_ID",
    "OriginRequestPolicyId": "b689b0a8-53d0-40ab-baf2-68738e2966ac"
  }
}
```

Notes on the config:
- `FUNCTION_URL_DOMAIN`: the hostname part of the Lambda function URL
  (e.g. `abcdef1234567890.lambda-url.us-east-1.on.aws`). Do not
  include `https://` or a trailing slash.
- `LAMBDA_OAC_ID`: the `Id` from the OAC you created in Step 1.
- `OriginRequestPolicyId`: `b689b0a8-53d0-40ab-baf2-68738e2966ac` is
  the AWS managed policy `AllViewerExceptHostHeader`. This forwards the
  `Accept` header and query strings to Lambda without forwarding the
  `Host` header (which would confuse Lambda function URL routing).
- `CUSTOM_CACHE_POLICY_ID`: see §2.7 below.

```bash
aws cloudfront create-distribution \
  --distribution-config file://dist-config.json
```

Add a CNAME or ALIAS record in your DNS:
```
cdn.astrophoto.pics  CNAME  <CloudFront domain>.cloudfront.net
```

### 2.7 Custom cache policy (query-string keyed)

**Do not use** `Managed-CachingOptimized` — it strips query strings
from the cache key, so `?w=400` and `?w=800` would return the same
cached object.

Create a custom cache policy that includes the five query params used
by the transformer:

```bash
aws cloudfront create-cache-policy \
  --cache-policy-config '{
    "Name": "astrophoto-image-transformer-cache",
    "Comment": "Includes w, h, fit, q, fm in cache key",
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

The `Vary: Accept` header on Lambda responses handles AVIF/WebP format
negotiation within a given `fm` param (or absent `fm`). Use the `Id`
returned here as `CUSTOM_CACHE_POLICY_ID` in the distribution config.

**Note:** The old project used `[width, format, quality]` as the
whitelist keys. The new URL shape uses `[w, h, fit, q, fm]` — use the
new names.

### 2.8 Smoke test

After DNS propagates and the distribution deploys (allow up to 15 min):

```bash
# First request — should be a cache miss, returns 200 image/jpeg
curl -I "https://cdn.astrophoto.pics/img/<known-photo-id>?w=400"

# Second request — should hit CloudFront cache
curl -I "https://cdn.astrophoto.pics/img/<known-photo-id>?w=400"
# Expect: x-cache: Hit from cloudfront
```

Check that different `w` values produce different cached objects:

```bash
curl -I "https://cdn.astrophoto.pics/img/<known-photo-id>?w=400"
curl -I "https://cdn.astrophoto.pics/img/<known-photo-id>?w=800"
# Both should return 200; the second should be a separate cache miss
# on first hit, showing the cache policy is keying on w correctly.
```

---

## 3. Production environment wiring

### 3.1 Backend env vars

Set these in the prod environment (`.env`, Kubernetes secret,
Doppler, or equivalent). No `APP_S3_ENDPOINT` — that variable is only
set in dev to point to MinIO. When unset, the `aws-sdk-s3` crate
resolves the standard AWS endpoint automatically.

```
APP_S3_REGION=us-east-1
APP_S3_BUCKET=astrophoto-images-prod
APP_S3_ACCESS_KEY=<AccessKeyId from astrophoto-prod-uploader>
APP_S3_SECRET_KEY=<SecretAccessKey from astrophoto-prod-uploader>
APP_S3_PATH_STYLE=false

# Do NOT set APP_S3_ENDPOINT — leave it unset for AWS endpoint resolution.

APP_CDN_BASE_URL=https://cdn.astrophoto.pics
```

`APP_S3_PATH_STYLE=false` enables virtual-hosted-style URLs
(`bucket.s3.amazonaws.com`) required by AWS S3. In dev, MinIO needs
`APP_S3_PATH_STYLE=true` because it uses path-style by default.

### 3.2 Frontend env vars

```
PUBLIC_CDN_BASE_URL=https://cdn.astrophoto.pics
VITE_API_BASE_URL=https://astrophoto.pics
```

`PUBLIC_CDN_BASE_URL` is the value read by the frontend `<Img>`
component (and the CDN URL builder) to construct image URLs. In dev
this is set to `http://localhost:8080/cdn` (the backend local route).

### 3.3 R2 → S3 migration (one-shot, conditional)

The current project (Phase 1 onwards) uses AWS S3 as its canonical
storage. This section applies only if an environment was previously
running with Cloudflare R2.

For a new prod deployment starting from Phase 1, this is a no-op —
skip this section.

**If migrating from an existing R2 deployment:**

```bash
# Prerequisites: rclone configured with two remotes:
#   r2-source  — Cloudflare R2 (S3-compatible endpoint)
#   s3-prod    — AWS S3 (standard)

# Copy originals
rclone copy r2-source:astrophoto-r2/originals/ \
  s3-prod:astrophoto-images-prod/originals/ \
  --progress \
  --transfers 8

# Copy any existing display masters (if they were stored in R2)
rclone copy r2-source:astrophoto-r2/display/ \
  s3-prod:astrophoto-images-prod/display/ \
  --progress \
  --transfers 8
```

If display masters were not stored in R2 (or if you want to regenerate
them from originals to ensure consistency with the new 4096 px spec):

```bash
# Re-derive display masters from all originals.
# This is a backend one-shot binary or admin endpoint — TBD as part
# of the prod deploy tooling:
cargo run --bin rederive-display-masters \
  --release -- --bucket astrophoto-images-prod --region us-east-1
```

**Decommission R2 only after:**
1. CloudFront is serving real traffic from S3 (verify with CloudFront
   access logs or `x-cache` headers on live traffic).
2. All photo URLs in the database resolve correctly via the new CDN.
3. A full backup of R2 originals is confirmed on S3.

---

## 4. Verification checklist

- [ ] Bucket `astrophoto-images-prod` exists in `us-east-1`.
- [ ] All four Block-Public-Access flags are `true`.
- [ ] Bucket ownership set to `BucketOwnerEnforced` (ACLs disabled).
- [ ] CORS allows `https://astrophoto.pics` and `https://www.astrophoto.pics` for `PUT`, `GET`, `HEAD`.
- [ ] Lifecycle rule aborts incomplete multipart uploads after 1 day.
- [ ] Bucket policy grants `s3:GetObject` on `display/*` to the Lambda execution role ARN.
- [ ] IAM user `astrophoto-prod-uploader` exists with scoped policy (no `PutObjectAcl`).
- [ ] Lambda function `astrophoto-image-transformer` deployed in `us-east-1`, runtime `nodejs20.x`.
- [ ] Lambda function URL `AuthType: AWS_IAM`.
- [ ] Lambda execution role has `s3:GetObject` on `display/*`.
- [ ] ACM certificate for `cdn.astrophoto.pics` in `us-east-1` status `ISSUED`.
- [ ] CloudFront OAC created with `OriginAccessControlOriginType: lambda`.
- [ ] CloudFront distribution has `cdn.astrophoto.pics` as alternate domain.
- [ ] Distribution origin domain is the Lambda function URL hostname (not an S3 bucket).
- [ ] `lambda:InvokeFunctionUrl` permission on function scoped to distribution ARN.
- [ ] Custom cache policy includes `w`, `h`, `fit`, `q`, `fm` in query-string whitelist.
- [ ] Origin request policy forwards `Accept` header to Lambda.
- [ ] DNS CNAME `cdn.astrophoto.pics` → CloudFront domain.
- [ ] `curl -I https://cdn.astrophoto.pics/img/<id>?w=400` returns `HTTP/2 200`.
- [ ] Second identical request returns `x-cache: Hit from cloudfront`.
- [ ] Backend env: `APP_S3_ENDPOINT` is **unset**.
- [ ] Backend env: `APP_S3_PATH_STYLE=false`.
- [ ] Backend env: `APP_CDN_BASE_URL=https://cdn.astrophoto.pics`.
- [ ] Frontend env: `PUBLIC_CDN_BASE_URL=https://cdn.astrophoto.pics`.

---

## 5. References

- Spec: `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
- Plan: `docs/superpowers/plans/2026-05-03-photographer-showcase-p1-foundations.md`
- Dev bucket reference: `astrophoto-images-dev` in `ap-southeast-1`
- Old project image-transformer source (Lambda@Edge shape, for transform logic only):
  `/Volumes/Pascal4Tb/Projects/claude/astrophoto/dev/current-lambda/image-transformer.js`
- AWS docs — Lambda function URLs:
  https://docs.aws.amazon.com/lambda/latest/dg/lambda-urls.html
- AWS docs — CloudFront OAC for Lambda function URLs:
  https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/private-content-restricting-access-to-lambda.html
- AWS Solutions Library — Serverless Image Handler (alternative to this
  custom implementation; evaluating it is outside Phase 1 scope):
  https://aws.amazon.com/solutions/implementations/serverless-image-handler/
