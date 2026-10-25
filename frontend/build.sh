#!/usr/bin/env bash
set -e

# ==============================================================================
# Fileyard Frontend Docker Build Script
# ==============================================================================

IMAGE_VERSION="${1:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="${IMAGE_NAME:-fileyard-frontend}"

if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
  echo "Usage: ./build.sh <image-version>"
  echo "Example: ./build.sh 1.0.0"
  echo "         IMAGE_NAME=my-registry.com/fileyard-frontend ./build.sh 1.0.0"
  exit 0
fi

if [ -z "$IMAGE_VERSION" ]; then
  echo "Error: Image version is required."
  echo "Usage: ./build.sh <image-version>"
  echo "Example: ./build.sh 1.0.0"
  exit 1
fi

echo "==> Building Docker image: ${IMAGE_NAME}:${IMAGE_VERSION} and ${IMAGE_NAME}:latest"
echo "==> Dockerfile: ${SCRIPT_DIR}/deploy/Dockerfile"
echo "==> Context:    ${SCRIPT_DIR}"

docker build \
  -t "${IMAGE_NAME}:${IMAGE_VERSION}" \
  -t "${IMAGE_NAME}:latest" \
  -f "${SCRIPT_DIR}/deploy/Dockerfile" \
  "${SCRIPT_DIR}"

echo "==> Successfully built ${IMAGE_NAME}:${IMAGE_VERSION}"