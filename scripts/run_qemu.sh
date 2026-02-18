#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$ROOT_DIR/build/os.img"

"$ROOT_DIR/scripts/build_image.sh"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "Missing required tool: qemu-system-x86_64" >&2
    exit 1
fi

qemu-system-x86_64 \
    -drive format=raw,file="$IMAGE" \
    -display none \
    -monitor none \
    -serial none \
    -debugcon stdio \
    -no-reboot \
    -no-shutdown
