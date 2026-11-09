#!/bin/sh
# Versioned authenticated GitHub release installer. gh owns authentication.
set -eu
repo=${WAYWR_REPOSITORY:-}
version=
prefix=${HOME:?}/.local/share/wayward-wrangler
select_default=true
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) version=${2:?}; shift 2;;
        --repo) repo=${2:?}; shift 2;;
        --prefix) prefix=${2:?}; shift 2;;
        --no-select-default) select_default=false; shift;;
        --help) printf '%s\n' 'install.sh --version vMAJOR.MINOR.PATCH --repo OWNER/REPO [--prefix ABSOLUTE] [--no-select-default]' 'WAYWR_REPOSITORY may supply the repository instead of --repo.'; exit 0;;
        *) echo 'Unknown installer option; use --help' >&2; exit 2;;
    esac
done
printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$' || { echo 'An explicit release version is required' >&2; exit 2; }
printf '%s\n' "$repo" | grep -Eq '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' || { echo 'Select the release repository with --repo OWNER/REPO or WAYWR_REPOSITORY' >&2; exit 2; }
case "$prefix" in /*) ;; *) echo 'Prefix must be absolute' >&2; exit 2;; esac
[ "$(uname -s)/$(uname -m)" = Linux/x86_64 ] || { echo 'No supported binary for this platform' >&2; exit 2; }
# The published baseline is Ubuntu 24.04 / glibc 2.39, not all Linux systems.
libc=$(getconf GNU_LIBC_VERSION 2>/dev/null) || { echo 'GNU libc is required' >&2; exit 2; }
libc_version=${libc#glibc }; major=${libc_version%%.*}; minor=${libc_version#*.}; minor=${minor%%.*}
[ "$major" -gt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -ge 39 ]; } || { echo 'Binary requires glibc 2.39 or newer' >&2; exit 2; }
command -v gh >/dev/null || { echo 'Install GitHub CLI and run gh auth login with repository read access first' >&2; exit 2; }
command -v sha256sum >/dev/null; command -v tar >/dev/null; command -v gzip >/dev/null
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
name="waywr-$version-linux-x86_64-gnu"
# Never embed tokens in URLs, logs, argv or a generated credential file.
tag=$(gh api "repos/$repo/releases/tags/$version" --jq .tag_name)
[ "$tag" = "$version" ] || { echo 'Release tag identity mismatch' >&2; exit 2; }
checksum_digest=$(gh api "repos/$repo/releases/tags/$version" --jq '.assets[] | select(.name == "SHA256SUMS") | .digest')
case "$checksum_digest" in sha256:*) expected=${checksum_digest#sha256:};; *) echo 'Release has no GitHub checksum asset digest; refusing unverifiable download' >&2; exit 2;; esac
[ "${#expected}" -eq 64 ] || exit 2
printf '%s' "$expected" | grep -Eq '^[0-9a-f]+$' || exit 2
gh release download "$version" --repo "$repo" --pattern SHA256SUMS --pattern "$name.tar.gz" --dir "$scratch"
printf '%s  %s\n' "$expected" "$scratch/SHA256SUMS" | sha256sum --check --status
# Accept exactly one checksum for the exact expected archive, with no paths.
[ "$(wc -l < "$scratch/SHA256SUMS" | tr -d ' ')" -eq 1 ] || exit 2
awk -v expected="$name.tar.gz" 'length($1)==64 && $1 !~ /[^0-9a-f]/ && $2==expected && NF==2 {ok=1} END {exit !ok}' "$scratch/SHA256SUMS"
(cd "$scratch" && sha256sum --check --status SHA256SUMS)
# Restrict archive names/types before extraction; reject links, devices and traversal.
tar -tzf "$scratch/$name.tar.gz" > "$scratch/names"
awk -v prefix="$name/" 'index($0,prefix)!=1 || $0 ~ /[^A-Za-z0-9_.\/-]/ || $0 ~ /\/\// || $0 ~ /(^|\/)\.\.?($|\/)/ {exit 1}' "$scratch/names"
[ "$(sort "$scratch/names" | uniq -d | wc -l)" -eq 0 ] || { echo 'Duplicate archive entries' >&2; exit 2; }
tar -tvzf "$scratch/$name.tar.gz" | awk 'substr($0,1,1)!="-" && substr($0,1,1)!="d" {exit 1}'
tar -xzf "$scratch/$name.tar.gz" --no-same-owner --no-same-permissions -C "$scratch"
set -- "$scratch/$name/bin/waywr" bundle-install --bundle "$scratch/$name" --prefix "$prefix"
if [ "$select_default" = true ]; then set -- "$@" --select-default; fi
"$@"
printf '%s\n' 'Installed. Existing project release bindings are unchanged.' 'Add ~/.local/bin to PATH if needed. Upgrade a selected project explicitly with:' "  waywr --project /absolute/project session upgrade --release $version"
