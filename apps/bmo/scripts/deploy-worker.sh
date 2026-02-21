#!/usr/bin/env bash
set -euo pipefail

REGION="ap-northeast-2"
ACCOUNT_ID="509399603331"
REPO="bmo-worker"
IMAGE="${ACCOUNT_ID}.dkr.ecr.${REGION}.amazonaws.com/${REPO}"

cd "$(dirname "$0")/.."

echo "==> Logging in to ECR..."
aws ecr get-login-password --region "${REGION}" | docker login --username AWS --password-stdin "${ACCOUNT_ID}.dkr.ecr.${REGION}.amazonaws.com"

echo "==> Building image..."
docker build --platform linux/arm64 -t "${REPO}" .

echo "==> Tagging and pushing..."
docker tag "${REPO}:latest" "${IMAGE}:latest"
docker push "${IMAGE}:latest"

echo "==> Updating Lambda function..."
aws lambda update-function-code \
  --region "${REGION}" \
  --function-name bmo-worker \
  --image-uri "${IMAGE}:latest" \
  > /dev/null

echo "Done."
