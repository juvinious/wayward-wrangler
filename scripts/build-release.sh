#!/bin/sh
# Remap the builder's checkout, home and toolchain paths in distributed binaries.
set -eu
set -f
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"
separator=$(printf '\037')
flags=${CARGO_ENCODED_RUSTFLAGS-}
if [ "${CARGO_ENCODED_RUSTFLAGS+x}" != x ]; then
    # Cargo uses whitespace splitting for RUSTFLAGS too. Encoded flags preserve
    # the remapping arguments below even when a local path contains spaces.
    for flag in ${RUSTFLAGS-}; do flags="${flags:+$flags$separator}$flag"; done
fi
toolchain=$(rustc --print sysroot)
for mapping in "${HOME:?}=/waywr/build-home" "$root=/waywr/source" "$toolchain=/waywr/toolchain"; do
    flags="${flags:+$flags$separator}--remap-path-prefix=$mapping"
done
export CARGO_ENCODED_RUSTFLAGS="$flags"
cargo build --locked --release --bin waywr --bin waywr-service --bin waywr-runner "$@"
