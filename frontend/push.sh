#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Fileyard Frontend Docker Push Script
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

REGISTRY="${REGISTRY:-crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box}"
IMAGE_NAME="${IMAGE_NAME:-fileyard-frontend}"
TARGET_REPO="${IMAGE_REPOSITORY:-${REGISTRY}/${IMAGE_NAME}}"

AUTO_YES=false
VERSION=""

# Parse arguments
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      echo "Usage: ./push.sh [VERSION] [-y|--yes]"
      echo ""
      echo "Examples:"
      echo "  ./push.sh                     # Auto-detect latest local version"
      echo "  ./push.sh 1.0.0               # Push version 1.0.0 and latest"
      echo "  ./push.sh 1.0.0 -y            # Push non-interactively"
      echo ""
      echo "Environment variables:"
      echo "  REGISTRY                      Image registry (Default: ${REGISTRY})"
      echo "  IMAGE_NAME                    Image name (Default: ${IMAGE_NAME})"
      echo "  IMAGE_REPOSITORY              Full target repo (Default: ${TARGET_REPO})"
      exit 0
      ;;
    -y|--yes)
      AUTO_YES=true
      ;;
    *)
      if [ -z "$VERSION" ]; then
        VERSION="$arg"
      fi
      ;;
  esac
done

if ! command -v docker >/dev/null 2>&1; then
  echo "Error: docker command not found. Please ensure Docker is running." >&2
  exit 1
fi

# If version is not specified, auto-detect latest local semver tag
if [ -z "$VERSION" ]; then
  DETECTED_VERSION=$(
    docker images --format '{{.Repository}}\t{{.Tag}}' |
      awk -F '\t' -v img="$IMAGE_NAME" -v repo="$TARGET_REPO" '
        ($1 == img || $1 == repo) && $2 != "latest" && $2 != "<none>" { print $2 }
      ' |
      sort -V |
      tail -n 1
  )

  if [ -n "$DETECTED_VERSION" ]; then
    VERSION="$DETECTED_VERSION"
    echo "Notice: No version specified. Auto-detected latest version: ${VERSION}"
  else
    if docker image inspect "${IMAGE_NAME}:latest" >/dev/null 2>&1 || docker image inspect "${TARGET_REPO}:latest" >/dev/null 2>&1; then
      VERSION="latest"
      echo "Notice: No version tag found. Using 'latest'."
    else
      echo "Error: No matching frontend images found for '${IMAGE_NAME}' or '${TARGET_REPO}'." >&2
      echo "Please build an image first: ./build.sh <version>" >&2
      exit 1
    fi
  fi
fi

echo "=================================================="
echo "Preparing to push Fileyard Frontend Docker image"
echo "Target Repository: ${TARGET_REPO}"
echo "Selected Version:  ${VERSION}"
echo "=================================================="

find_source_image() {
  local tag="$1"
  if docker image inspect "${TARGET_REPO}:${tag}" >/dev/null 2>&1; then
    echo "${TARGET_REPO}:${tag}"
  elif docker image inspect "${IMAGE_NAME}:${tag}" >/dev/null 2>&1; then
    echo "${IMAGE_NAME}:${tag}"
  else
    echo ""
  fi
}

VERSION_IMAGE=$(find_source_image "$VERSION")
LATEST_IMAGE=$(find_source_image "latest")

if [ -z "$VERSION_IMAGE" ]; then
  echo "" >&2
  echo "Error: Could not find image for version '${VERSION}' locally." >&2
  echo "Please build it first with: ./build.sh ${VERSION}" >&2
  echo "" >&2
  echo "Current matching local images:" >&2
  docker images | grep -E "(${IMAGE_NAME}|yaobo-box)" || true
  exit 1
fi

IMAGES_TO_PUSH=()
IMAGES_TO_PUSH+=("${VERSION_IMAGE} -> ${TARGET_REPO}:${VERSION}")

PUSH_LATEST=false
if [ "$VERSION" != "latest" ] && [ -n "$LATEST_IMAGE" ]; then
  PUSH_LATEST=true
  IMAGES_TO_PUSH+=("${LATEST_IMAGE} -> ${TARGET_REPO}:latest")
fi

echo ""
echo "Images to push:"
for item in "${IMAGES_TO_PUSH[@]}"; do
  echo "  - ${item}"
done
echo ""

if [ "$AUTO_YES" = false ]; then
  read -r -p "Confirm push to remote registry? [y/N] " answer
  case "$answer" in
    y|Y|yes|YES|Yes) ;;
    *)
      echo "Push cancelled."
      exit 0
      ;;
  esac
fi

push_tag() {
  local src="$1"
  local target="$2"

  if [ "$src" != "$target" ]; then
    echo "==> Tagging: ${src} -> ${target}"
    docker tag "$src" "$target"
  fi

  echo "==> Pushing: ${target}"
  docker push "$target"
}

echo ""
# Push versioned image
push_tag "$VERSION_IMAGE" "${TARGET_REPO}:${VERSION}"

# Push latest image if available
if [ "$PUSH_LATEST" = true ]; then
  push_tag "$LATEST_IMAGE" "${TARGET_REPO}:latest"
fi

echo ""
echo "=================================================="
echo "Frontend image push completed successfully!"
echo "Pushed: ${TARGET_REPO}:${VERSION}"
if [ "$PUSH_LATEST" = true ]; then
  echo "Pushed: ${TARGET_REPO}:latest"
fi
echo "=================================================="
