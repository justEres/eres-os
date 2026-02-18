#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/build"
TARGET_DIR="$ROOT_DIR/target/x86_64-unknown-none/release"
CARGO_FEATURES="${ERES_FEATURES:-}"

mkdir -p "$BUILD_DIR"

for cmd in cargo as ld ld.lld; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Missing required tool: $cmd" >&2
        exit 1
    fi
done

if command -v llvm-objcopy >/dev/null 2>&1; then
    OBJCOPY=llvm-objcopy
elif command -v objcopy >/dev/null 2>&1; then
    OBJCOPY=objcopy
else
    echo "Missing required tool: llvm-objcopy or objcopy" >&2
    exit 1
fi

if ! rustup target list --installed | grep -qx "x86_64-unknown-none"; then
    rustup target add x86_64-unknown-none
fi

if [[ -n "$CARGO_FEATURES" ]]; then
    cargo build --release --target x86_64-unknown-none --features "$CARGO_FEATURES"
else
    cargo build --release --target x86_64-unknown-none
fi

as --64 "$ROOT_DIR/boot/stage2.S" -o "$BUILD_DIR/stage2.o"

ld.lld \
    -m elf_x86_64 \
    -nostdlib \
    --gc-sections \
    -T "$ROOT_DIR/build/linker.ld" \
    -o "$BUILD_DIR/stage2.elf" \
    "$BUILD_DIR/stage2.o" \
    "$TARGET_DIR/liberes_os.a"

"$OBJCOPY" -O binary "$BUILD_DIR/stage2.elf" "$BUILD_DIR/stage2.bin"

stage2_size=$(stat -c%s "$BUILD_DIR/stage2.bin")
stage2_sectors=$(((stage2_size + 511) / 512))

if (( stage2_sectors == 0 )); then
    echo "Invalid stage2 size: $stage2_size bytes" >&2
    exit 1
fi

as --32 --defsym STAGE2_SECTORS="$stage2_sectors" "$ROOT_DIR/boot/boot.S" -o "$BUILD_DIR/boot.o"

ld \
    -m elf_i386 \
    -nostdlib \
    -N \
    --oformat binary \
    -Ttext 0x7c00 \
    -e _start \
    -o "$BUILD_DIR/boot.bin" \
    "$BUILD_DIR/boot.o"

img_sectors=$((1 + stage2_sectors))
dd if=/dev/zero of="$BUILD_DIR/os.img" bs=512 count="$img_sectors" status=none
dd if="$BUILD_DIR/boot.bin" of="$BUILD_DIR/os.img" conv=notrunc status=none
dd if="$BUILD_DIR/stage2.bin" of="$BUILD_DIR/os.img" bs=512 seek=1 conv=notrunc status=none

echo "Built $BUILD_DIR/os.img"
echo "Stage2 size: $stage2_size bytes ($stage2_sectors sectors)"
