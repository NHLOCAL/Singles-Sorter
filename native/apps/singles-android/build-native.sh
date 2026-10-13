#!/usr/bin/env bash
# Existing pinned sources, official NDK r27d. Outputs must be outside Git.
set -euo pipefail
: "${ANDROID_NDK_HOME:?Official NDK path required}"
: "${PCRE2_SOURCE:?Verified PCRE2 10.46 source required}"
: "${NATIVE_OUT:?Fresh absolute output directory required}"
[[ "$NATIVE_OUT" = /* && ! -e "$NATIVE_OUT" ]] || { echo 'NATIVE_OUT must be fresh and absolute' >&2; exit 1; }
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../.." && pwd -P)
case "$NATIVE_OUT/" in "$REPO_ROOT/"*) echo 'Build outputs must be outside Git' >&2;exit 1;;esac
mkdir -p "$NATIVE_OUT"
export SINGLES_SORTER_ANDROID_PRODUCT=1 SINGLES_SORTER_ANDROID_PROBE=1
export CARGO_TARGET_DIR="$NATIVE_OUT/target"
for pair in x86_64:x86_64-linux-android arm64-v8a:aarch64-linux-android; do
    ABI=${pair%%:*}; TARGET=${pair#*:}
    PCRE="$NATIVE_OUT/pcre-$ABI"
    cmake -S "$PCRE2_SOURCE" -B "$PCRE" \
        -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake" \
        -DANDROID_ABI="$ABI" -DANDROID_PLATFORM=android-29 \
        -DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON -DCMAKE_BUILD_TYPE=Release \
        -DPCRE2_BUILD_PCRE2_8=OFF -DPCRE2_BUILD_PCRE2_16=OFF -DPCRE2_BUILD_PCRE2_32=ON \
        -DBUILD_SHARED_LIBS=OFF -DPCRE2_STATIC_PIC=ON -DPCRE2_SUPPORT_UNICODE=ON \
        -DPCRE2_SUPPORT_JIT=OFF -DPCRE2_BUILD_PCRE2GREP=OFF -DPCRE2_BUILD_TESTS=OFF
    cmake --build "$PCRE" --target pcre2-32-static --parallel 2
    export PCRE2_STATIC_DIR="$PCRE"
    LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/${TARGET}29-clang"
    VAR="CARGO_TARGET_${TARGET^^}_LINKER"; VAR=${VAR//-/_}; export "$VAR=$LINKER"
    export RUSTFLAGS='-C link-arg=-Wl,-z,max-page-size=16384'
    cargo +1.99.0 build --locked --release --manifest-path "$REPO_ROOT/native/plugins/original-inference/Cargo.toml" --features product --target "$TARGET"
    mkdir -p "$NATIVE_OUT/$ABI"
    cp "$CARGO_TARGET_DIR/$TARGET/release/libsingles_android_inference.so" "$NATIVE_OUT/$ABI/"
    "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf" -lW "$NATIVE_OUT/$ABI/libsingles_android_inference.so" > "$NATIVE_OUT/$ABI/elf-program-headers.txt"
    python3 "$SCRIPT_DIR/check-android-elf.py" \
        --readelf "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf" \
        --library "$NATIVE_OUT/$ABI/libsingles_android_inference.so" --abi "$ABI" \
        --out "$NATIVE_OUT/$ABI/elf-linkage.json"
    "$LINKER" -Wall -Wextra -Werror -fPIE -pie -Wl,-z,max-page-size=16384 \
        "$SCRIPT_DIR/native-load-probe.c" -ldl -o "$NATIVE_OUT/$ABI/native-load-probe"
done
