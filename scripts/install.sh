#!/bin/sh
# Public HTTPS release installer. Authenticated gh is used only with --private.
set -eu
umask 077
repo=${WAYWR_REPOSITORY:-}
version=
prefix=${HOME:?}/.local/share/wayward-wrangler
select_default=true
private=false
fail() { printf '%s\n' "$1" >&2; exit 2; }
valid_version() {
    case "$1" in *[!A-Za-z0-9.-]*) return 1;; esac
    [ "${#1}" -le 64 ] && printf '%s\n' "$1" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$'
}
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) version=${2:?}; shift 2;;
        --repo) repo=${2:?}; shift 2;;
        --prefix) prefix=${2:?}; shift 2;;
        --no-select-default) select_default=false; shift;;
        --private) private=true; shift;;
        --help) printf '%s\n' 'install.sh --repo OWNER/REPO [--version vMAJOR.MINOR.PATCH] [--prefix ABSOLUTE] [--no-select-default] [--private]' 'Installs the latest stable public release by default; no GitHub login is needed.' 'WAYWR_REPOSITORY may supply the repository instead of --repo.' '--private explicitly uses authenticated GitHub CLI for private releases.'; exit 0;;
        *) fail 'Unknown installer option; use --help';;
    esac
done
printf '%s\n' "$repo" | grep -Eq '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' || fail 'Select the release repository with --repo OWNER/REPO or WAYWR_REPOSITORY'
case "$repo" in ./*|../*|*/.|*/..) fail 'Invalid release repository';; esac
if [ -n "$version" ]; then valid_version "$version" || fail 'Invalid release version'; fi
case "$prefix" in /*) ;; *) fail 'Prefix must be absolute';; esac
[ "$(uname -s)/$(uname -m)" = Linux/x86_64 ] || fail 'No supported binary for this platform'
# The binary baseline is Ubuntu 24.04 / glibc 2.39, not all Linux systems.
libc=$(getconf GNU_LIBC_VERSION 2>/dev/null) || fail 'GNU libc is required'
libc_version=${libc#glibc }; major=${libc_version%%.*}; minor=${libc_version#*.}; minor=${minor%%.*}
[ "$major" -gt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -ge 39 ]; } || fail 'Binary requires glibc 2.39 or newer'
command -v node >/dev/null || fail 'Install Node >=22.19 before installing Waywr (node must be on PATH)'
node -e 'const [major,minor]=process.versions.node.split(".").map(Number); process.exit(major>22 || major===22 && minor>=19 ? 0 : 1)' 2>/dev/null || fail 'Install a working Node >=22.19 before installing Waywr'
for tool in git flock sha256sum tar gzip; do
    command -v "$tool" >/dev/null || fail "Install the required runtime/installation tool: $tool"
done
if [ "$private" = true ]; then
    command -v gh >/dev/null || fail '--private requires GitHub CLI and gh auth login with repository read access'
else
    command -v curl >/dev/null || fail 'Install curl to download the public release'
fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT HUP INT TERM
fetch() {
    # Ignore curlrc: public installs must not inherit account/authentication options.
    download_url=$1; download_output=$2; shift 2
    curl -q --proto '=https' --proto-redir '=https' --tlsv1.2 --fail --silent --show-error --location \
        --retry 3 --connect-timeout 15 --max-time 300 "$@" --output "$download_output" "$download_url"
}
selector=latest
if [ -n "$version" ]; then selector="tags/$version"; fi
if [ "$private" = true ]; then
    gh api "repos/$repo/releases/$selector" > "$scratch/release.json"
else
    fetch "https://api.github.com/repos/$repo/releases/$selector" "$scratch/release.json" \
        --header 'Accept: application/vnd.github+json' --header 'X-GitHub-Api-Version: 2026-03-10' || fail 'Public release metadata unavailable; check the repository, published release and network. Private repositories require --private.'
fi
# JSON is parsed as data, never shell code. Download URLs are constructed below
# from the validated repository/tag rather than trusting URLs in the response.
node - "$scratch" "$version" <<'NODE'
const fs = require('node:fs');
const [directory, requested] = process.argv.slice(2);
try {
    let release;
    try { release = JSON.parse(fs.readFileSync(directory + '/release.json', 'utf8')); }
    catch { throw new Error('Invalid release metadata'); }
    const tag = release.tag_name;
    if (typeof tag !== 'string' || tag.length > 64 || tag !== tag.trim() || !/^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$/.test(tag))
        throw new Error('Invalid release version in metadata');
    if (requested && tag !== requested) throw new Error('Release tag identity mismatch');
    if (release.draft !== false || !requested && release.prerelease !== false)
        throw new Error('Latest installation requires a published stable release');
    if (!Array.isArray(release.assets)) throw new Error('Release assets missing');
    const asset = name => {
        const matches = release.assets.filter(item => item && item.name === name);
        if (matches.length !== 1) throw new Error('Required release asset missing or duplicated');
        return matches[0];
    };
    asset('waywr-' + tag + '-linux-x86_64-gnu.tar.gz');
    const digest = asset('SHA256SUMS').digest;
    if (typeof digest !== 'string' || !/^sha256:[0-9a-f]{64}$/.test(digest))
        throw new Error('Release has no valid GitHub checksum asset digest; refusing unverifiable download');
    fs.writeFileSync(directory + '/version', tag);
    fs.writeFileSync(directory + '/checksum-digest', digest.slice(7));
} catch (error) {
    console.error(error.message);
    process.exit(2);
}
NODE
version=$(cat "$scratch/version")
expected=$(cat "$scratch/checksum-digest")
name="waywr-$version-linux-x86_64-gnu"
if [ "$private" = true ]; then
    gh release download "$version" --repo "$repo" --pattern SHA256SUMS --pattern "$name.tar.gz" --dir "$scratch"
else
    fetch "https://github.com/$repo/releases/download/$version/SHA256SUMS" "$scratch/SHA256SUMS" || fail 'Release checksum download failed; existing installation unchanged'
    fetch "https://github.com/$repo/releases/download/$version/$name.tar.gz" "$scratch/$name.tar.gz" || fail 'Release archive download failed; existing installation unchanged'
fi
printf '%s  %s\n' "$expected" "$scratch/SHA256SUMS" | sha256sum --check --status || fail 'GitHub checksum asset digest mismatch; existing installation unchanged'
# Accept exactly one checksum for the exact expected archive, with no paths.
[ "$(wc -l < "$scratch/SHA256SUMS" | tr -d ' ')" -eq 1 ] || fail 'Invalid release checksum list'
awk -v expected="$name.tar.gz" 'length($1)==64 && $1 !~ /[^0-9a-f]/ && $2==expected && NF==2 {ok=1} END {exit !ok}' "$scratch/SHA256SUMS" || fail 'Invalid release archive checksum entry'
(cd "$scratch" && sha256sum --check --status SHA256SUMS) || fail 'Release archive checksum mismatch; existing installation unchanged'
# Restrict archive names/types before extraction; reject links, devices and traversal.
tar -tzf "$scratch/$name.tar.gz" > "$scratch/names"
awk -v prefix="$name/" 'index($0,prefix)!=1 || $0 ~ /[^A-Za-z0-9_.\/-]/ || $0 ~ /\/\// || $0 ~ /(^|\/)\.\.?($|\/)/ {exit 1}' "$scratch/names" || fail 'Unsafe archive path'
[ "$(sort "$scratch/names" | uniq -d | wc -l)" -eq 0 ] || fail 'Duplicate archive entries'
tar -tvzf "$scratch/$name.tar.gz" | awk 'substr($0,1,1)!="-" && substr($0,1,1)!="d" {exit 1}' || fail 'Unsafe archive entry type'
tar -xzf "$scratch/$name.tar.gz" --no-same-owner --no-same-permissions -C "$scratch"
set -- "$scratch/$name/bin/waywr" bundle-install --bundle "$scratch/$name" --prefix "$prefix"
if [ "$select_default" = true ]; then set -- "$@" --select-default; fi
"$@"
printf '%s\n' "Installed Waywr $version"
if [ "$select_default" = true ]; then
    printf '%s\n' "Launcher: $prefix/bin/waywr" 'For this Bash session, if the launcher is not on PATH:'
    printf '  export PATH='
    printf "'"; printf '%s' "$prefix/bin" | sed "s/'/'\\\\''/g"; printf "'"
    printf ':"$PATH"\n'
    printf '%s\n' 'Open your project and run waywr. No per-project application upgrade is needed.'
else
    printf '%s\n' "Release executable: $prefix/releases/$version/bin/waywr" 'Default application selection was not changed.'
fi
