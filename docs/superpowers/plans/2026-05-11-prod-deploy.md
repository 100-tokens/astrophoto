# Production deploy stream implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up Astrophoto production on Koyeb (web + api + Postgres) with `astrophoto.pics` resolving via Vercel DNS, image storage on AWS S3 fronted by CloudFront + Lambda@Edge, and tag-based promotion from `main`.

**Architecture:** Two Koyeb compute services (`astrophoto-prod-web` for SvelteKit SSR, `astrophoto-prod` for the Rust backend) plus a Koyeb managed Postgres. AWS holds the image bucket `astrophoto-images-prod`, a Lambda@Edge sharp-resize handler, and a CloudFront distribution aliased to `cdn.astrophoto.pics`. Vercel acts solely as authoritative DNS for the `astrophoto.pics` zone — no Vercel runtime in the request path.

**Tech Stack:** AWS CLI (S3, IAM, Lambda, ACM, CloudFront), `koyeb` CLI / dashboard, Vercel DNS dashboard, Google Cloud Console (OAuth client), Node 20 + sharp (Lambda@Edge handler), the repo's existing Rust/SvelteKit code.

**Spec:** `docs/superpowers/specs/2026-05-11-prod-deploy-design.md`. The AWS provisioning commands are sourced from `docs/operations/aws-s3-cloudfront.md`; this plan inlines the prod-specific overlays.

**Conventions used below:**
- Verification steps end every task. If the verification doesn't match the expected output, stop and investigate before proceeding.
- Commits are made for any artifacts that land in the repo (DNS values, Lambda source if you opt to commit it). AWS / Koyeb / Vercel state lives in those providers, not the repo, and isn't committed.
- `ACCOUNT_ID` below means `$(aws sts get-caller-identity --query Account --output text)`. Capture it once into your shell.

---

## Task 1: Create prod S3 bucket with public access blocked

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Create the bucket in `us-east-1`**

```bash
aws s3api create-bucket \
  --bucket astrophoto-images-prod \
  --region us-east-1
```

Expected: JSON output with `"Location": "/astrophoto-images-prod"`.

- [ ] **Step 2: Block all public access**

```bash
aws s3api put-public-access-block \
  --bucket astrophoto-images-prod \
  --public-access-block-configuration \
    BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true
```

- [ ] **Step 3: Enforce bucket-owner ownership (disable ACLs)**

```bash
aws s3api put-bucket-ownership-controls \
  --bucket astrophoto-images-prod \
  --ownership-controls 'Rules=[{ObjectOwnership=BucketOwnerEnforced}]'
```

- [ ] **Step 4: Apply lifecycle for stale multipart uploads**

Create `/tmp/prod-lifecycle.json`:

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
  --bucket astrophoto-images-prod \
  --lifecycle-configuration file:///tmp/prod-lifecycle.json
```

- [ ] **Step 5: Apply CORS for browser presigned PUTs**

Create `/tmp/prod-cors.json`:

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
  --bucket astrophoto-images-prod \
  --cors-configuration file:///tmp/prod-cors.json
```

- [ ] **Step 6: Verify**

```bash
aws s3api get-public-access-block --bucket astrophoto-images-prod
aws s3api get-bucket-cors --bucket astrophoto-images-prod
aws s3api get-bucket-lifecycle-configuration --bucket astrophoto-images-prod
```

Expected: all four public-access fields `true`; CORS shows the two astrophoto.pics origins; lifecycle shows `abort-stale-multiparts`.

---

## Task 2: Create the prod uploader IAM user

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Create the user**

```bash
aws iam create-user --user-name astrophoto-prod-uploader
```

- [ ] **Step 2: Write the inline policy**

Create `/tmp/prod-uploader-policy.json`:

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

- [ ] **Step 3: Attach the policy**

```bash
aws iam put-user-policy \
  --user-name astrophoto-prod-uploader \
  --policy-name astrophoto-prod-uploader-policy \
  --policy-document file:///tmp/prod-uploader-policy.json
```

- [ ] **Step 4: Create an access key and capture credentials**

```bash
aws iam create-access-key --user-name astrophoto-prod-uploader
```

Capture `AccessKeyId` and `SecretAccessKey` into a temporary, password-manager-only note. Do **not** paste them into the shell history file, `.env`, or any file in the repo. They will be entered directly into Koyeb's secret store in Task 14.

- [ ] **Step 5: Verify**

```bash
aws iam get-user-policy \
  --user-name astrophoto-prod-uploader \
  --policy-name astrophoto-prod-uploader-policy
```

Expected: the policy document round-trips with the two statements above.

---

## Task 3: Build the prod Lambda@Edge artifact

**Files:** Build in `/tmp/astrophoto-prod-lambda/` (mirrors the staging convention). Not committed to the repo for parity with how the staging Lambda was built.

- [ ] **Step 1: Create the build directory and the handler source**

```bash
mkdir -p /tmp/astrophoto-prod-lambda
cd /tmp/astrophoto-prod-lambda
```

Write `/tmp/astrophoto-prod-lambda/index.cjs`:

```javascript
'use strict';

const sharp = require('sharp');
const querystring = require('querystring');
const { S3Client, GetObjectCommand } = require('@aws-sdk/client-s3');

const BUCKET = 'astrophoto-images-prod';
const REGION = 'us-east-1';
const s3 = new S3Client({ region: REGION });

exports.handler = async (event) => {
  const { request } = event.Records[0].cf;

  if (request.method !== 'GET' && request.method !== 'HEAD') {
    return request;
  }

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
    console.error('image-transformer error:', e && e.message);
    return request;
  }
};
```

This is byte-identical to the staging handler except `BUCKET = 'astrophoto-images-prod'`.

- [ ] **Step 2: Install sharp with Linux x86_64 binaries**

```bash
cd /tmp/astrophoto-prod-lambda
npm init -y
npm install sharp @aws-sdk/client-s3 --cpu=x64 --os=linux --include=optional
```

Expected: `node_modules/sharp/build/Release/sharp-linux-x64.node` exists.

- [ ] **Step 3: Zip the deployable**

```bash
cd /tmp/astrophoto-prod-lambda
zip -r function.zip index.cjs node_modules/
ls -lh function.zip
```

Expected: `function.zip` is roughly 35–60 MB.

---

## Task 4: Create the prod Lambda execution role

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Create the role with both trusted principals**

```bash
aws iam create-role \
  --role-name astrophoto-prod-lambda-exec \
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

The `edgelambda.amazonaws.com` principal is mandatory; without it CloudFront cannot replicate the function to edge locations.

- [ ] **Step 2: Attach CloudWatch Logs**

```bash
aws iam attach-role-policy \
  --role-name astrophoto-prod-lambda-exec \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole
```

- [ ] **Step 3: Grant `s3:GetObject` on `display/*`**

```bash
aws iam put-role-policy \
  --role-name astrophoto-prod-lambda-exec \
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

- [ ] **Step 4: Verify**

```bash
aws iam get-role --role-name astrophoto-prod-lambda-exec \
  --query 'Role.AssumeRolePolicyDocument.Statement[0].Principal.Service'
```

Expected: array containing `"lambda.amazonaws.com"` and `"edgelambda.amazonaws.com"`.

---

## Task 5: Deploy the prod Lambda function and publish a version

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Capture the role ARN**

```bash
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
ROLE_ARN="arn:aws:iam::${ACCOUNT_ID}:role/astrophoto-prod-lambda-exec"
echo "$ROLE_ARN"
```

- [ ] **Step 2: Create the Lambda function**

```bash
aws lambda create-function \
  --function-name astrophoto-prod-image-transformer \
  --runtime nodejs20.x \
  --role "$ROLE_ARN" \
  --handler index.handler \
  --zip-file fileb:///tmp/astrophoto-prod-lambda/function.zip \
  --timeout 10 \
  --memory-size 1024 \
  --region us-east-1
```

Do **not** pass `--environment`. Lambda@Edge does not allow env vars; the bucket name is baked into the handler.

- [ ] **Step 3: Publish a version**

```bash
aws lambda publish-version \
  --function-name astrophoto-prod-image-transformer \
  --region us-east-1
```

Capture the `Version` integer in the output. The versioned ARN is:

```
arn:aws:lambda:us-east-1:${ACCOUNT_ID}:function:astrophoto-prod-image-transformer:<VERSION>
```

Save this string — it's needed in Task 9 (CloudFront distribution config).

- [ ] **Step 4: Verify**

```bash
aws lambda get-function-configuration \
  --function-name astrophoto-prod-image-transformer:<VERSION> \
  --region us-east-1 \
  --query '[State, LastUpdateStatus]'
```

Expected: `["Active", "Successful"]`.

---

## Task 6: Request an ACM certificate for `cdn.astrophoto.pics`

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Request the cert in `us-east-1` (CloudFront requirement)**

```bash
aws acm request-certificate \
  --domain-name cdn.astrophoto.pics \
  --subject-alternative-names "*.astrophoto.pics" \
  --validation-method DNS \
  --region us-east-1
```

Capture the `CertificateArn`. The `*.astrophoto.pics` SAN is included so the same cert can later cover `api.` or other subdomains if you ever decide to put them behind CloudFront too.

- [ ] **Step 2: Read the DNS validation records**

```bash
aws acm describe-certificate \
  --certificate-arn <ARN> \
  --region us-east-1 \
  --query 'Certificate.DomainValidationOptions[].[DomainName, ResourceRecord.Name, ResourceRecord.Value]'
```

Expected: two rows (one for the apex-domain SAN, one for `cdn.`) each showing a CNAME name + value to add at the DNS provider.

Save the output verbatim — Task 7 uses it.

---

## Task 7: Add the ACM validation CNAMEs at Vercel

**Files:** None in repo. Vercel state only.

- [ ] **Step 1: Add each validation record at Vercel**

In the Vercel dashboard for the `astrophoto.pics` domain, under *DNS Records*, add one CNAME for each row from Task 6 Step 2. The hostname is the long random `_…` name from ACM; the value is the ACM target ending in `.acm-validations.aws`.

Note: ACM produces names with the apex appended (e.g., `_abc123.astrophoto.pics.`). At Vercel, enter only the relative portion (`_abc123`).

- [ ] **Step 2: Wait for ACM to issue**

Validation typically completes in under 5 minutes once DNS propagates.

```bash
aws acm describe-certificate \
  --certificate-arn <ARN> \
  --region us-east-1 \
  --query 'Certificate.Status'
```

Expected: `"ISSUED"`. If it stays at `"PENDING_VALIDATION"` for more than 15 minutes, re-check the Vercel records against the ACM-provided names exactly.

---

## Task 8: Create the S3 OAC and CloudFront cache policy

If staging already has an OAC named `astrophoto-s3-oac` and a cache policy named `astrophoto-image-transformer-cache`, you can reuse both for prod. Otherwise create them.

- [ ] **Step 1: Check whether the OAC already exists**

```bash
aws cloudfront list-origin-access-controls \
  --query "OriginAccessControlList.Items[?Name=='astrophoto-s3-oac'].Id"
```

If the array is empty, run Step 2; otherwise capture the existing `Id` and skip to Step 3.

- [ ] **Step 2: Create the OAC**

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

Capture the `Id` field. Save as `S3_OAC_ID` for Task 9.

- [ ] **Step 3: Check whether the cache policy already exists**

```bash
aws cloudfront list-cache-policies --type custom \
  --query "CachePolicyList.Items[?CachePolicy.CachePolicyConfig.Name=='astrophoto-image-transformer-cache'].CachePolicy.Id"
```

If empty, run Step 4; otherwise capture the existing `Id`.

- [ ] **Step 4: Create the cache policy**

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

Capture the `Id`. Save as `CACHE_POLICY_ID` for Task 9.

---

## Task 9: Create the prod CloudFront distribution

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Write the distribution config**

Create `/tmp/prod-dist-config.json`, substituting `ACCOUNT_ID`, the versioned Lambda ARN from Task 5, `S3_OAC_ID` from Task 8, `CACHE_POLICY_ID` from Task 8, and the ACM cert ARN from Task 6:

```json
{
  "CallerReference": "astrophoto-prod-cf-1",
  "Comment": "Astrophoto prod image CDN",
  "Enabled": true,
  "HttpVersion": "http2and3",
  "IsIPV6Enabled": true,
  "PriceClass": "PriceClass_100",
  "Aliases": { "Quantity": 1, "Items": ["cdn.astrophoto.pics"] },
  "ViewerCertificate": {
    "ACMCertificateArn": "<ACM_CERT_ARN>",
    "SSLSupportMethod": "sni-only",
    "MinimumProtocolVersion": "TLSv1.2_2021"
  },
  "Origins": {
    "Quantity": 1,
    "Items": [
      {
        "Id": "s3-display-origin",
        "DomainName": "astrophoto-images-prod.s3.us-east-1.amazonaws.com",
        "S3OriginConfig": { "OriginAccessIdentity": "" },
        "OriginAccessControlId": "<S3_OAC_ID>"
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
    "CachePolicyId": "<CACHE_POLICY_ID>",
    "LambdaFunctionAssociations": {
      "Quantity": 1,
      "Items": [
        {
          "LambdaFunctionARN": "<VERSIONED_LAMBDA_ARN>",
          "EventType": "origin-request",
          "IncludeBody": false
        }
      ]
    }
  }
}
```

`CallerReference` must be unique per attempt — change the suffix (`astrophoto-prod-cf-2`, etc.) if you have to retry.

- [ ] **Step 2: Create the distribution**

```bash
aws cloudfront create-distribution \
  --distribution-config file:///tmp/prod-dist-config.json
```

Capture two values from the output:
- `Distribution.Id` (call this `DIST_ID`)
- `Distribution.DomainName` (the `*.cloudfront.net` hostname — call this `DIST_DOMAIN`)

- [ ] **Step 3: Verify the distribution exists**

```bash
aws cloudfront get-distribution --id <DIST_ID> --query 'Distribution.Status'
```

Expected: `"InProgress"` (first deploy takes 10–20 min) and eventually `"Deployed"`.

---

## Task 10: Apply the S3 bucket policy granting CloudFront OAC access

**Files:** None in repo. AWS state only.

- [ ] **Step 1: Write the bucket policy**

Create `/tmp/prod-bucket-policy.json`, substituting `ACCOUNT_ID` and `DIST_ID`:

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
          "aws:SourceArn": "arn:aws:cloudfront::<ACCOUNT_ID>:distribution/<DIST_ID>"
        }
      }
    }
  ]
}
```

- [ ] **Step 2: Apply**

```bash
aws s3api put-bucket-policy \
  --bucket astrophoto-images-prod \
  --policy file:///tmp/prod-bucket-policy.json
```

- [ ] **Step 3: Verify**

```bash
aws s3api get-bucket-policy --bucket astrophoto-images-prod \
  --query Policy --output text | python3 -m json.tool
```

Expected: the policy round-trips with the correct `aws:SourceArn` value.

---

## Task 11: Wait for the CloudFront distribution to reach `Deployed`

**Files:** None in repo.

- [ ] **Step 1: Poll**

```bash
aws cloudfront get-distribution --id <DIST_ID> --query 'Distribution.Status'
```

Repeat every couple of minutes until the output is `"Deployed"`. This typically takes 10–20 minutes on first create.

- [ ] **Step 2: Quick origin-pull sanity check**

The Lambda regex `^/img/([0-9a-f-]+)$` requires hex-only names, so pick a hex placeholder like `cafebabe`.

Upload a test JPEG to the prod bucket:

```bash
aws s3 cp /path/to/any.jpg s3://astrophoto-images-prod/display/cafebabe.jpg
```

Then hit the CloudFront domain directly (we haven't added the `cdn.` CNAME yet):

```bash
curl -sI "https://<DIST_DOMAIN>/img/cafebabe?w=200&h=200&fm=jpeg"
```

Expected: `HTTP/2 200`, `Content-Type: image/jpeg`. If you get `403`, the bucket policy in Task 10 didn't apply correctly. If you get `502`, the Lambda failed — check CloudWatch Logs. Lambda@Edge logs land in the AWS region nearest your client, not always `us-east-1`. Check the log group `/aws/lambda/<region>.astrophoto-prod-image-transformer` where `<region>` is the edge region that served your request (visible in `x-amz-cf-pop` response header).

- [ ] **Step 3: Clean up the test object**

```bash
aws s3 rm s3://astrophoto-images-prod/display/cafebabe.jpg
```

---

## Task 12: Provision the Koyeb prod app and managed Postgres

**Files:** None in repo. Koyeb state only.

- [ ] **Step 1: Create a Koyeb app named `astrophoto-prod`**

In the Koyeb dashboard: *Create App → Name: `astrophoto-prod`*. (Or via CLI: `koyeb app create astrophoto-prod`.) This is the parent app that will hold both the backend and the web services.

- [ ] **Step 2: Create the managed Postgres database**

In the Koyeb dashboard: *Databases → Create Database*. Settings:

- Name: `astrophoto-prod-db`
- Engine: PostgreSQL 16
- Plan: smallest paid Koyeb Postgres tier (NOT the free tier — it lacks PITR and is capacity-constrained)
- Region: same region as the staging database (verify in the staging app's dashboard before selecting)
- Backups: daily (default on paid plans)

- [ ] **Step 3: Capture the connection string**

After provisioning, the database detail page shows a "Connection string" — it's a `postgres://…` URL.

Note: Koyeb services can reference this via `${{ astrophoto-prod-db.DATABASE_URL }}` when the DB is in the same app context, which avoids ever pasting the string into a secret. You'll use that reference syntax in Task 15. No need to copy the raw URL.

- [ ] **Step 4: Verify**

In the Koyeb dashboard, the database status should read `Available`.

---

## Task 13: Create the prod Google OAuth client

**Files:** None in repo. Google Cloud Console state only.

- [ ] **Step 1: In Google Cloud Console, create a new OAuth 2.0 Client ID**

*APIs & Services → Credentials → Create Credentials → OAuth client ID*.

- Application type: Web application
- Name: `Astrophoto Prod`
- Authorized JavaScript origins: `https://astrophoto.pics`
- Authorized redirect URIs: `https://api.astrophoto.pics/auth/google/callback`

- [ ] **Step 2: Capture the Client ID and Client Secret**

These will be loaded into Koyeb secrets in Task 14. Do **not** check them in.

- [ ] **Step 3: Verify the consent screen is published (or in test mode with the prod owner whitelisted)**

The staging client likely uses *Testing* mode, which limits sign-ins to whitelisted test users. For prod, either *Publish* the OAuth consent screen (requires Google verification for >100 users) or keep it in Testing mode for the soft launch and plan verification as a follow-up.

---

## Task 14: Create the Koyeb prod secrets

**Files:** None in repo. Koyeb state only.

- [ ] **Step 1: Generate a fresh session signing key**

```bash
openssl rand -base64 32
```

Capture the output. Do not reuse the staging key.

- [ ] **Step 2: In the Koyeb dashboard, create the following secrets**

Under *Secrets → Create*. Name each exactly as listed; the values go to the providers indicated:

| Secret name                       | Value source                                                  |
|-----------------------------------|---------------------------------------------------------------|
| `prod_aws_access_key_id`          | `AccessKeyId` from Task 2 Step 4                              |
| `prod_aws_secret_access_key`      | `SecretAccessKey` from Task 2 Step 4                          |
| `prod_session_signing_key`        | Output of `openssl rand -base64 32` from Step 1               |
| `prod_google_oauth_client_id`     | Client ID from Task 13                                        |
| `prod_google_oauth_client_secret` | Client Secret from Task 13                                    |
| `prod_postmark_token`             | New transactional-email token (Postmark or whichever provider staging uses; create a new prod token) |

The `DATABASE_URL` is **not** a separate secret — it's referenced inline from the DB service in Task 15 (`${{ astrophoto-prod-db.DATABASE_URL }}`).

- [ ] **Step 3: Verify**

In the Koyeb dashboard, all six secrets above are listed under *Secrets*.

---

## Task 15: Create the Koyeb backend service (`astrophoto-prod`)

**Files:** None in repo. Koyeb state only.

- [ ] **Step 1: Inside the `astrophoto-prod` app, create a Service**

Settings:
- Service type: Web
- Service name: `astrophoto-prod`
- Source: GitHub repo (the same one staging deploys from)
- Branch/Tag filter: tags matching `v*` (NOT `main`)
- Build method: Dockerfile (same path used by staging — confirm by looking at staging's service settings before saving)
- Instance type: `small` (1 vCPU, 1 GB) — CPU is the constraint due to `spawn_blocking` image decode
- Region: same as the Postgres in Task 12 (Frankfurt or Washington — match staging)
- Health check: HTTP `GET /healthz` on the service's exposed port (the staging service's port; usually 8080 — confirm)
- Public route: `/` proxied to the service port; will be assigned a `*.koyeb.app` hostname (capture this; call it `BACKEND_KOYEB_HOST`)

- [ ] **Step 2: Set environment variables**

Plain env (not secrets):

```
APP_ENV=prod
APP_BASE_URL=https://astrophoto.pics
APP_CORS_ORIGIN=https://astrophoto.pics
APP_S3_BUCKET=astrophoto-images-prod
APP_S3_REGION=us-east-1
APP_S3_PATH_STYLE=false
APP_CDN_BASE_URL=https://cdn.astrophoto.pics
```

Do NOT set `APP_S3_ENDPOINT` or `APP_CDN_LOCAL_FALLBACK`. Both must be absent.

Secret references (use the dropdown in the Koyeb env-var UI):

```
DATABASE_URL              = ${{ astrophoto-prod-db.DATABASE_URL }}
AWS_ACCESS_KEY_ID         = {{ secret.prod_aws_access_key_id }}
AWS_SECRET_ACCESS_KEY     = {{ secret.prod_aws_secret_access_key }}
APP_SESSION_SIGNING_KEY   = {{ secret.prod_session_signing_key }}
APP_GOOGLE_OAUTH_CLIENT_ID     = {{ secret.prod_google_oauth_client_id }}
APP_GOOGLE_OAUTH_CLIENT_SECRET = {{ secret.prod_google_oauth_client_secret }}
APP_POSTMARK_TOKEN        = {{ secret.prod_postmark_token }}
```

The exact env-var names on the right side of `=` must match the names the Rust code reads. If your staging service uses different names (e.g., `APP_GOOGLE_CLIENT_ID` without the `OAUTH` suffix), match those exactly — verify against the staging service's env list before saving.

- [ ] **Step 3: Add the custom domain (do not switch DNS yet)**

In the service's *Domains* tab, add `api.astrophoto.pics`. Koyeb will display a verification CNAME (a `_koyeb-verification-...` record).

Capture this verification record. Task 18 adds it at Vercel.

- [ ] **Step 4: Save the service in a paused state**

Save the service settings but do **not** start deployments yet (or set the tag filter to a non-existent tag like `v0.0.0-pending` so it won't deploy until Task 20). We don't want a service running without DNS in place.

---

## Task 16: Create the Koyeb web service (`astrophoto-prod-web`)

**Files:** None in repo. Koyeb state only.

- [ ] **Step 1: Inside the `astrophoto-prod` app, create a second Service**

Settings:
- Service type: Web
- Service name: `astrophoto-prod-web`
- Source: same GitHub repo
- Branch/Tag filter: tags matching `v*`
- Build method: Buildpack (Node 22); build command `pnpm install --frozen-lockfile && pnpm build`; run command `node build/index.js`
- Instance type: `nano` (1 vCPU, 512 MB)
- Region: same region as the backend
- Health check: HTTP `GET /` returns 200
- Public route: `/` proxied to port 3000 (SvelteKit's default); capture the assigned `*.koyeb.app` hostname (call it `WEB_KOYEB_HOST`)

- [ ] **Step 2: Set environment variables**

Plain env:

```
PUBLIC_API_BASE_URL=https://api.astrophoto.pics
PUBLIC_CDN_BASE_URL=https://cdn.astrophoto.pics
ORIGIN=https://astrophoto.pics
```

(`ORIGIN` is required by SvelteKit's Node adapter when behind a reverse proxy, to set the canonical URL.)

No secrets are needed on the web service — it talks to the backend via public HTTPS, not directly to S3 or the DB.

- [ ] **Step 3: Add the custom domains**

In the *Domains* tab, add **both** `astrophoto.pics` and `www.astrophoto.pics`. Koyeb shows verification CNAMEs for each. Capture both.

- [ ] **Step 4: Save in a paused state (same pattern as Task 15 Step 4)**

---

## Task 17: Find Koyeb's apex IPs for the A records

**Files:** None in repo.

- [ ] **Step 1: Read Koyeb's published anycast IPs**

In the Koyeb dashboard, under the web service's *Domains → astrophoto.pics → DNS instructions*, Koyeb shows the exact A-record values to use for the apex. There are typically two anycast IPs.

Capture both. They go into Vercel in Task 18.

If the dashboard only shows a CNAME (no apex A records), this means the Koyeb account is using their newer CNAME-only flow — in that case, drop the A records from the plan and treat the apex as a CNAME (Vercel DNS supports apex CNAMEs via flattening). Document whichever path the dashboard prescribes.

---

## Task 18: Add the DNS records at Vercel

**Files:** None in repo. Vercel state only.

- [ ] **Step 1: Add Koyeb domain-verification CNAMEs**

In Vercel DNS for `astrophoto.pics`, add the three verification CNAMEs captured in Tasks 15 Step 3 and 16 Step 3:

- `_koyeb-verification-<random>` for `api.astrophoto.pics` (CNAME → Koyeb-provided value)
- `_koyeb-verification-<random>` for `astrophoto.pics` (CNAME → Koyeb-provided value)
- `_koyeb-verification-<random>` for `www.astrophoto.pics` (CNAME → Koyeb-provided value)

At Vercel, the host field for the apex verification record is just the random portion (no trailing `.astrophoto.pics`).

- [ ] **Step 2: Add the apex A records**

For `astrophoto.pics` (host = `@` or blank, depending on Vercel UI), add A records pointing to the Koyeb anycast IPs from Task 17.

- [ ] **Step 3: Add `www.astrophoto.pics`**

Add `www` as a CNAME → `astrophoto.pics`. (Koyeb will issue a 301 redirect to the apex once the domain is verified there.)

- [ ] **Step 4: Add `api.astrophoto.pics`**

Add `api` as a CNAME → `<BACKEND_KOYEB_HOST>` (the `*.koyeb.app` hostname captured in Task 15 Step 1).

- [ ] **Step 5: Add `cdn.astrophoto.pics`**

Add `cdn` as a CNAME → `<DIST_DOMAIN>` (the CloudFront hostname captured in Task 9 Step 2).

- [ ] **Step 6: Verify with dig**

```bash
dig +short astrophoto.pics
dig +short www.astrophoto.pics
dig +short api.astrophoto.pics
dig +short cdn.astrophoto.pics
```

Expected: each returns the corresponding target IP/hostname. Propagation usually takes under a minute on Vercel DNS but allow up to 15.

---

## Task 19: Verify Koyeb domain ownership and wait for Let's Encrypt

**Files:** None in repo.

- [ ] **Step 1: Trigger verification in Koyeb**

In the Koyeb dashboard, for each of the three custom domains (`api.astrophoto.pics`, `astrophoto.pics`, `www.astrophoto.pics`), click *Verify* (or wait for Koyeb's auto-verify to pick up the CNAMEs).

- [ ] **Step 2: Wait for Let's Encrypt provisioning**

Once a domain is verified, Koyeb issues a Let's Encrypt cert automatically. This typically takes 1–5 minutes per domain.

- [ ] **Step 3: Verify**

```bash
curl -sI https://astrophoto.pics/
curl -sI https://www.astrophoto.pics/
curl -sI https://api.astrophoto.pics/healthz
```

Expected: all three return TLS-valid responses. They will return 502 or similar at this stage because no service is running — that's fine. What matters is that the TLS handshake succeeds.

If you get a TLS error, the cert hasn't provisioned yet; wait another few minutes.

---

## Task 20: Tag a release and push

**Files:** None in repo (the tag is git state, not file state).

- [ ] **Step 1: Ensure `main` is at the commit you want in prod**

```bash
git fetch origin
git checkout main
git pull --ff-only
git log -1 --oneline
```

Confirm the head commit is what you expect (and that it has been soaking on staging — visit the staging URL and re-verify the upload + view flow before tagging).

- [ ] **Step 2: Tag the release**

```bash
TAG="v$(date +%Y.%m.%d)"
git tag -a "$TAG" -m "Production release $TAG"
git push origin "$TAG"
```

(If you've already cut a tag today, suffix it: `v2026.05.11-2`.)

- [ ] **Step 3: Re-enable deployments on the two Koyeb services**

In each service's *Settings → Source*, change the tag filter from the paused placeholder to `v*` (or specifically to the tag you just pushed). Save — this triggers a build.

- [ ] **Step 4: Watch the builds**

In the Koyeb dashboard, both `astrophoto-prod` and `astrophoto-prod-web` should show a build in progress. Watch the logs.

Expected: backend build runs `sqlx::migrate!()` on first boot against the empty prod DB. If migrations fail, stop immediately and investigate before proceeding.

- [ ] **Step 5: Verify health**

```bash
curl -s https://api.astrophoto.pics/healthz
curl -sI https://astrophoto.pics/
```

Expected: `/healthz` returns the healthy response (whatever the backend returns — `200 OK` body `{"status":"ok"}` or similar; match against staging). The web service returns `200 OK` for `/`.

---

## Task 21: Production smoke test

**Files:** None in repo.

This is the gating step before you announce the URL. Run the full happy path in order; stop at the first failure.

- [ ] **Step 1: Signup with email**

In a fresh incognito window, navigate to `https://astrophoto.pics/signup`. Create an account using a real email you control. Submit.

Expected: confirmation email arrives within ~60s. Click the confirmation link → redirect to a signed-in state on the production frontend.

- [ ] **Step 2: Sign out, sign back in with email + password**

Sign out, then sign in via `https://astrophoto.pics/signin` using the credentials from Step 1.

Expected: successful sign-in, session cookie `__Host-session` set in the browser (Application tab in DevTools) with `Secure`, `SameSite=None`, `HttpOnly`.

- [ ] **Step 3: Sign in with Google OAuth**

Sign out, click *Sign in with Google*. Complete the OAuth flow.

Expected: redirect through `api.astrophoto.pics/auth/google/callback` and back to `astrophoto.pics`, signed in. If you used a brand-new Google account that wasn't on the staging client's whitelist, this is the test that proves the prod OAuth client (Task 13) is correctly configured.

- [ ] **Step 4: Upload a photo**

Navigate to `https://astrophoto.pics/upload`. Pick a JPEG (5–15 MB; representative of a real astrophotograph).

Expected:
- Browser dev tools show a `PUT` to `https://astrophoto-images-prod.s3.us-east-1.amazonaws.com/originals/<photo-id>...` returning `200`.
- The follow-up `/api/photos/finalize` call returns `200`.
- The photo page renders within a few seconds (the `display/<photo-id>.jpg` master is generated in `spawn_blocking`).

- [ ] **Step 5: Verify the photo viewer pulls through CloudFront**

On the photo page, inspect the `<img>` src. It should be `https://cdn.astrophoto.pics/img/<photo-id>?w=...&fm=jpeg` (or similar).

Hit the URL directly:

```bash
curl -sI "https://cdn.astrophoto.pics/img/<photo-id>?w=800&h=800&fm=jpeg"
```

Expected: `HTTP/2 200`, `Content-Type: image/jpeg`, `x-cache: Miss from cloudfront` on the first request and `Hit from cloudfront` on the second.

- [ ] **Step 6: Delete the photo**

Delete the test photo via the UI.

Expected: the photo disappears from the profile page; an `aws s3 ls s3://astrophoto-images-prod/originals/ | grep <photo-id>` returns empty.

- [ ] **Step 7: Final read-back**

```bash
curl -sI https://astrophoto.pics/
curl -sI https://api.astrophoto.pics/healthz
curl -sI "https://cdn.astrophoto.pics/img/00000000-0000-0000-0000-000000000000?w=200&fm=jpeg"
```

Expected: first two `200 OK`. The third is a hex-only name that matches the Lambda regex but doesn't exist in S3: the handler attempts `GetObject` on `display/00000000-…jpg`, catches the `NoSuchKey` error, and falls through to origin. CloudFront then forwards the original `/img/00000000-…` URI to S3, which is outside the `display/*` OAC grant — so the final HTTP response is `403`. (A `200` here would be alarming: it would mean the bucket somehow served the wrong key. A `502` indicates the Lambda crashed before reaching the catch block.)

---

## Done criteria

- All 21 tasks have their verification steps satisfied.
- `https://astrophoto.pics`, `https://api.astrophoto.pics`, `https://cdn.astrophoto.pics` all serve correctly with valid TLS.
- The smoke test in Task 21 passes end-to-end on a freshly created account.
- The two Koyeb services are deployed against the same git tag.

## Open follow-ups (out of scope for this plan)

- Add a GitHub Actions workflow that creates the tag only after `just check` and the test suite pass (deferred — see spec).
- Move the Lambda@Edge source into the repo under `infra/lambda-edge/image-transformer/` with a build script that templates `BUCKET` for staging/prod (deferred — staging built the same ad-hoc way; consolidation is a separate clean-up).
- Publish the OAuth consent screen for >100 users / non-test traffic.
- Add `docs/operations/runbook-prod.md` with first-incident-response notes once a real incident has been worked.
