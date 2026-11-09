#!/bin/sh
# Build/select one local paired application. No service, model or remote Git write.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
prefix=${HOME:?}/.local/share/wayward-wrangler
while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix) prefix=${2:?Specify an absolute installation prefix}; shift 2;;
        --help) printf '%s\n' 'sh scripts/dev-install.sh [--prefix ABSOLUTE]' 'Builds this checkout and selects the paired local application.' 'Preserves project work, existing releases and shell configuration.'; exit 0;;
        *) echo 'Unknown option; use --help' >&2; exit 2;;
    esac
done
case "$prefix" in /*) ;; *) echo 'Installation prefix must be absolute' >&2; exit 2;; esac
[ "$(uname -s)/$(uname -m)" = Linux/x86_64 ] || { echo 'Local installation currently supports Linux x86_64' >&2; exit 2; }
[ -x "$HOME/.cargo/bin/cargo" ] || { echo 'Install permanent rustup in ~/.cargo/bin first; see README source prerequisites' >&2; exit 2; }
PATH="$HOME/.cargo/bin:$PATH"
export PATH
for tool in git python3 rustc; do command -v "$tool" >/dev/null || { printf 'Missing source-build prerequisite: %s\n' "$tool" >&2; exit 2; }; done
cd "$root"
revision=$(git rev-parse --short=12 HEAD)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/waywr-dev.build.XXXXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
nonce=${scratch##*.}
release="dev-$(date -u +%Y%m%dT%H%M%SZ)-$revision-$nonce"
printf '%s\n' 'Building the paired Waywr application…'
sh "$root/scripts/build-release.sh"
cargo fetch --locked
python3 "$root/scripts/generate-notices.py" --check
python3 "$root/scripts/check-public-content.py" --artifact "$root/target/release/waywr" --artifact "$root/target/release/waywr-service" --artifact "$root/target/release/waywr-runner"
"$root/target/release/waywr" bundle-build --release-id "$release" \
    --output "$scratch/bundle" --source-root "$root" \
    --frontend "$root/target/release/waywr" --service "$root/target/release/waywr-service" \
    --runner "$root/target/release/waywr-runner"
python3 "$root/scripts/check-public-content.py" --artifact "$scratch/bundle"
"$root/target/release/waywr" bundle-install --bundle "$scratch/bundle" --prefix "$prefix" --select-default
"$prefix/bin/waywr" --version
printf '%s\n' "Launcher: $prefix/bin/waywr" 'Project work/settings are preserved. Close the old view and run waywr from your project.'
if [ "$prefix" = "$HOME/.local/share/wayward-wrangler" ] && ! command -v waywr >/dev/null 2>&1; then
    printf '%s\n' 'For Fish, add the launcher to PATH once:' '  fish_add_path "$HOME/.local/bin"'
fi
