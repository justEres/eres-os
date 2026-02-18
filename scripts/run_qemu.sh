#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$ROOT_DIR/build/os.img"
HEADLESS=0

usage() {
    cat <<'EOF'
Usage: ./scripts/run_qemu.sh [--headless]

Options:
  --headless   Run without a GUI window and print debug console output.
  -h, --help   Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --headless)
            HEADLESS=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

"$ROOT_DIR/scripts/build_image.sh"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "Missing required tool: qemu-system-x86_64" >&2
    exit 1
fi

if [[ "$HEADLESS" -eq 1 ]]; then
    qemu-system-x86_64 \
        -drive format=raw,file="$IMAGE" \
        -display none \
        -monitor none \
        -serial none \
        -debugcon stdio \
        -no-reboot \
        -no-shutdown
else
    qemu-system-x86_64 \
        -drive format=raw,file="$IMAGE" \
        -no-reboot \
        -no-shutdown
fi
