#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$ROOT_DIR/build/os.img"
FS_IMAGE="$ROOT_DIR/build/simplefs.img"
HEADLESS=0
TEST_MODE=0

usage() {
    cat <<'EOF'
Usage: ./scripts/run_qemu.sh [--headless] [--test]

Options:
  --headless   Run without a GUI window and print debug console output.
  --test       Build with qemu-test feature and exit with command-test result.
  -h, --help   Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --headless)
            HEADLESS=1
            shift
            ;;
        --test)
            TEST_MODE=1
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

if [[ "$TEST_MODE" -eq 1 ]]; then
    ERES_FEATURES="qemu-test" "$ROOT_DIR/scripts/build_image.sh"
else
    "$ROOT_DIR/scripts/build_image.sh"
fi

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "Missing required tool: qemu-system-x86_64" >&2
    exit 1
fi

if [[ "$HEADLESS" -eq 1 ]]; then
    QEMU_ARGS=(
        -drive "if=ide,index=0,media=disk,format=raw,file=$IMAGE"
        -display none
        -monitor none
        -serial none
        -debugcon stdio
        -no-reboot
        -no-shutdown
    )
    if [[ -f "$FS_IMAGE" ]]; then
        QEMU_ARGS+=(-drive "if=ide,index=1,media=disk,format=raw,file=$FS_IMAGE")
    fi

    if [[ "$TEST_MODE" -eq 1 ]]; then
        set +e
        output="$(timeout 10s qemu-system-x86_64 "${QEMU_ARGS[@]}" 2>&1)"
        status=$?
        set -e
        printf "%s\n" "$output"

        if grep -q "Eres OS: command tests OK." <<<"$output"; then
            exit 0
        fi

        echo "Command tests did not report success (qemu status: $status)." >&2
        exit 1
    else
        qemu-system-x86_64 "${QEMU_ARGS[@]}"
    fi
else
    QEMU_ARGS=(
        -drive "if=ide,index=0,media=disk,format=raw,file=$IMAGE"
        -no-reboot
        -no-shutdown
    )
    if [[ -f "$FS_IMAGE" ]]; then
        QEMU_ARGS+=(-drive "if=ide,index=1,media=disk,format=raw,file=$FS_IMAGE")
    fi
    qemu-system-x86_64 "${QEMU_ARGS[@]}"
fi
