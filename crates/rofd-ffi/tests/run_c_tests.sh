#!/bin/sh
set -eu

ROFD_REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
export ROFD_REPO_ROOT

INCLUDE_DIR="$ROFD_REPO_ROOT/crates/rofd-ffi/include"
TEST_DIR="$ROFD_REPO_ROOT/crates/rofd-ffi/tests/c"
ROFD_FFI_TARGET_DIR="$ROFD_REPO_ROOT/target/rofd-ffi-c-tests"
ROFD_RUSTC_VERSION=$(rustc -vV)
ROFD_HOST_TRIPLE=$(printf '%s\n' "$ROFD_RUSTC_VERSION" |
    sed -n 's/^host: //p')
ROFD_HOST_COUNT=$(printf '%s\n' "$ROFD_RUSTC_VERSION" |
    awk '/^host: / { count++ } END { print count + 0 }')

if [ "$ROFD_HOST_COUNT" -ne 1 ] || [ -z "$ROFD_HOST_TRIPLE" ]; then
    echo "could not determine a unique host triple from rustc -vV" >&2
    exit 1
fi

ROFD_FFI_DEBUG_DIR="$ROFD_FFI_TARGET_DIR/$ROFD_HOST_TRIPLE/debug"
ROFD_FFI_LIBRARY="$ROFD_FFI_DEBUG_DIR/librofd_ffi.so"
ROFD_C_HEADER_BIN="$ROFD_FFI_DEBUG_DIR/rofd-ffi-header-c"
ROFD_CPP_HEADER_BIN="$ROFD_FFI_DEBUG_DIR/rofd-ffi-header-cpp"
ROFD_SMOKE_BIN="$ROFD_FFI_DEBUG_DIR/rofd-ffi-smoke"
ROFD_NAVIGATION_BIN="$ROFD_FFI_DEBUG_DIR/rofd-ffi-navigation-smoke"
ROFD_NAVIGATION_FIXTURE="$ROFD_FFI_DEBUG_DIR/rofd-ffi-navigation.ofd"
ROFD_SYMBOLS_FILE="$ROFD_FFI_DEBUG_DIR/rofd-ffi-symbols.txt"

cargo build -p rofd-ffi --manifest-path "$ROFD_REPO_ROOT/Cargo.toml" \
    --target-dir "$ROFD_FFI_TARGET_DIR" --target "$ROFD_HOST_TRIPLE"
if [ ! -f "$ROFD_FFI_LIBRARY" ]; then
    echo "expected cdylib was not built at $ROFD_FFI_LIBRARY" >&2
    exit 1
fi

# The cdylib carries a versioned SONAME, so test binaries need the
# versioned name to resolve at runtime; provide it like ldconfig would.
ROFD_FFI_SONAME=$(readelf -d "$ROFD_FFI_LIBRARY" | sed -n 's/^.*Library soname: \[\(.*\)\]$/\1/p')
if [ -n "$ROFD_FFI_SONAME" ] && [ "$ROFD_FFI_SONAME" != "librofd_ffi.so" ]; then
    ln -sf librofd_ffi.so "$ROFD_FFI_DEBUG_DIR/$ROFD_FFI_SONAME"
fi

cc -std=c11 -Wall -Wextra -Werror -pedantic $(pkg-config --cflags cairo) \
    -I"$INCLUDE_DIR" "$TEST_DIR/header_compile.c" -L"$ROFD_FFI_DEBUG_DIR" \
    -lrofd_ffi $(pkg-config --libs cairo) -Wl,-rpath,"$ROFD_FFI_DEBUG_DIR" \
    -o "$ROFD_C_HEADER_BIN"
c++ -std=c++17 -Wall -Wextra -Werror -pedantic $(pkg-config --cflags cairo) \
    -I"$INCLUDE_DIR" "$TEST_DIR/header_compile.cpp" -L"$ROFD_FFI_DEBUG_DIR" \
    -lrofd_ffi $(pkg-config --libs cairo) -Wl,-rpath,"$ROFD_FFI_DEBUG_DIR" \
    -o "$ROFD_CPP_HEADER_BIN"

"$ROFD_C_HEADER_BIN"
"$ROFD_CPP_HEADER_BIN"

cc -std=c11 -Wall -Wextra -Werror -pedantic $(pkg-config --cflags cairo) \
    -I"$INCLUDE_DIR" "$TEST_DIR/ffi_smoke.c" -L"$ROFD_FFI_DEBUG_DIR" \
    -lrofd_ffi $(pkg-config --libs cairo) -Wl,-rpath,"$ROFD_FFI_DEBUG_DIR" \
    -o "$ROFD_SMOKE_BIN"
"$ROFD_SMOKE_BIN" "$ROFD_REPO_ROOT/learning/test.ofd"

# Generate a controlled package using the existing Rust ZIP test dependency,
# then verify nonempty versioned records through the newly built dynamic ABI.
cargo run -p rofd-ffi --example abi-fixtures --manifest-path "$ROFD_REPO_ROOT/Cargo.toml" \
    --target-dir "$ROFD_FFI_TARGET_DIR" --target "$ROFD_HOST_TRIPLE" -- "$ROFD_NAVIGATION_FIXTURE"
cc -std=c11 -Wall -Wextra -Werror -pedantic $(pkg-config --cflags cairo) \
    -I"$INCLUDE_DIR" "$TEST_DIR/navigation_smoke.c" -L"$ROFD_FFI_DEBUG_DIR" \
    -lrofd_ffi $(pkg-config --libs cairo) -Wl,-rpath,"$ROFD_FFI_DEBUG_DIR" \
    -o "$ROFD_NAVIGATION_BIN"
"$ROFD_NAVIGATION_BIN" "$ROFD_NAVIGATION_FIXTURE"

nm -D --defined-only "$ROFD_FFI_LIBRARY" | awk '{print $3}' | \
    sed -n '/^rofd_/p' | LC_ALL=C sort > "$ROFD_SYMBOLS_FILE"
diff -u "$TEST_DIR/expected-symbols.txt" "$ROFD_SYMBOLS_FILE"
