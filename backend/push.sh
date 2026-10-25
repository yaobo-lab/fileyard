#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Fileyard Backend Docker Push Script
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 默认仓库配置（支持环境变量覆盖）
REGISTRY="${REGISTRY:-crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box}"
IMAGE_NAME="${IMAGE_NAME:-fileyard-backend}"
TARGET_REPO="${IMAGE_REPOSITORY:-${REGISTRY}/${IMAGE_NAME}}"

# 默认从 crates/app/Cargo.toml 中提取版本号（与 build.sh 保持一致）
DEFAULT_VERSION=$(grep -m1 '^version' "$SCRIPT_DIR/crates/app/Cargo.toml" 2>/dev/null | sed -E 's/.*version = "(.*)"/\1/' || echo "")

AUTO_YES=false
VERSION=""

# 参数解析
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      echo "用法: ./push.sh [版本号] [-y|--yes]"
      echo ""
      echo "示例:"
      echo "  ./push.sh                     # 自动使用 Cargo.toml 版本 (${DEFAULT_VERSION:-latest})"
      echo "  ./push.sh 1.0.0               # 推送指定版本 1.0.0"
      echo "  ./push.sh 1.0.0 -y            # 指定版本并跳过确认提示"
      echo ""
      echo "环境变量:"
      echo "  REGISTRY                      镜像仓库地址 (默认: ${REGISTRY})"
      echo "  IMAGE_NAME                    镜像名称 (默认: ${IMAGE_NAME})"
      echo "  IMAGE_REPOSITORY              完整仓库路径 (默认: ${TARGET_REPO})"
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

VERSION="${VERSION:-${DEFAULT_VERSION:-latest}}"

if ! command -v docker >/dev/null 2>&1; then
  echo "错误: 未找到 docker 命令，请确认 Docker 已正确安装并运行。" >&2
  exit 1
fi

echo "=================================================="
echo "准备推送 Fileyard 后端 Docker 镜像"
echo "目标仓库: ${TARGET_REPO}"
echo "指定版本: ${VERSION}"
echo "=================================================="

# 查找本地可用的镜像源
# 优先匹配带完整仓库前缀的镜像（由 build.sh 生成），其次匹配短名称镜像
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

# 如果本地未找到指定版本的镜像
if [ -z "$VERSION_IMAGE" ]; then
  if [ -n "$LATEST_IMAGE" ]; then
    echo "提示: 本地未找到标签为 '${VERSION}' 的镜像，将使用本地 latest 镜像打标并推送。"
    VERSION_IMAGE="$LATEST_IMAGE"
  else
    echo "" >&2
    echo "错误: 未在本地找到版本为 '${VERSION}' 或 'latest' 的后端镜像！" >&2
    echo "请先运行构建脚本: ./build.sh ${VERSION}" >&2
    echo "" >&2
    echo "当前相关的本地镜像:" >&2
    docker images | grep -E "(${IMAGE_NAME}|yaobo-box)" || true
    exit 1
  fi
fi

IMAGES_TO_PUSH=()
IMAGES_TO_PUSH+=("${VERSION_IMAGE} -> ${TARGET_REPO}:${VERSION}")

PUSH_LATEST=false
if [ "$VERSION" != "latest" ] && [ -n "$LATEST_IMAGE" ]; then
  PUSH_LATEST=true
  IMAGES_TO_PUSH+=("${LATEST_IMAGE} -> ${TARGET_REPO}:latest")
fi

echo ""
echo "待推送的镜像列表:"
for item in "${IMAGES_TO_PUSH[@]}"; do
  echo "  - ${item}"
done
echo ""

if [ "$AUTO_YES" = false ]; then
  read -r -p "确认推送以上镜像到远程仓库? [y/N] " answer
  case "$answer" in
    y|Y|yes|YES|Yes) ;;
    *)
      echo "推送已取消。"
      exit 0
      ;;
  esac
fi

push_tag() {
  local src="$1"
  local target="$2"

  if [ "$src" != "$target" ]; then
    echo "==> 打标: ${src} -> ${target}"
    docker tag "$src" "$target"
  fi

  echo "==> 推送: ${target}"
  docker push "$target"
}

echo ""
# 推送指定版本镜像
push_tag "$VERSION_IMAGE" "${TARGET_REPO}:${VERSION}"

# 若存在 latest 镜像，同步推送 latest
if [ "$PUSH_LATEST" = true ]; then
  push_tag "$LATEST_IMAGE" "${TARGET_REPO}:latest"
fi

echo ""
echo "=================================================="
echo "后端镜像推送成功！"
echo "远程镜像: ${TARGET_REPO}:${VERSION}"
if [ "$PUSH_LATEST" = true ]; then
  echo "远程镜像: ${TARGET_REPO}:latest"
fi
echo "=================================================="
