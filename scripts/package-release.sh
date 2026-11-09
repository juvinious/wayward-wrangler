#!/bin/sh
# GNU/Linux x86_64 release assembly. No installation or publication.
set -eu
version=${1:?usage: package-release.sh VERSION OUTPUT_DIRECTORY}
out=${2:?usage: package-release.sh VERSION OUTPUT_DIRECTORY}
case "$version" in *[!A-Za-z0-9._-]*|'') echo 'Invalid release ID' >&2; exit 2;; esac
root=$(git rev-parse --show-toplevel)
case "$out" in /*) ;; *) out="$PWD/$out";; esac
[ ! -e "$out" ] || { echo 'Output must not exist' >&2; exit 2; }
[ "$(uname -s)/$(uname -m)" = Linux/x86_64 ] || { echo 'Packaging currently supports Linux x86_64 only' >&2; exit 2; }
cd "$root"
sh scripts/build-release.sh
cargo fetch --locked
python3 scripts/check-public-content.py --artifact target/release/waywr --artifact target/release/waywr-service --artifact target/release/waywr-runner
python3 scripts/generate-notices.py --check
mkdir -p "$out"
bundle="waywr-$version-linux-x86_64-gnu"
./target/release/waywr bundle-build --release-id "$version" --output "$out/$bundle" --source-root "$root" --frontend "$root/target/release/waywr" --service "$root/target/release/waywr-service" --runner "$root/target/release/waywr-runner"
python3 scripts/check-public-content.py --artifact "$out/$bundle"
epoch=$(git show -s --format=%ct HEAD)
tar --sort=name --mtime="@$epoch" --owner=0 --group=0 --numeric-owner --mode='u+rwX,go+rX' -C "$out" -cf - "$bundle" | gzip -n -9 > "$out/$bundle.tar.gz"
(cd "$out" && sha256sum "$bundle.tar.gz" > SHA256SUMS)
printf '%s\n' "$out/$bundle.tar.gz"
