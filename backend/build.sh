#!/usr/bin/env sh

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# 默认从 crates/app/Cargo.toml 中提取版本号，如传入参数则以参数为准
default_version=$(grep -m1 '^version' "$script_dir/crates/app/Cargo.toml" | sed -E 's/.*version = "(.*)"/\1/')
version="${1:-${default_version:-latest}}"

if ! command -v docker >/dev/null 2>&1; then
    echo "Error: docker command was not found." >&2
    exit 1
fi

image_repository="${IMAGE_REPOSITORY:-crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box/fileyard-backend}"
image="${image_repository}:${version}"

echo "Building backend image: ${image}"

docker build \
    --tag "$image" \
    --tag "${image_repository}:latest" \
    "$script_dir"

echo "Build completed: ${image}"
echo "Also tagged as: ${image_repository}:latest"