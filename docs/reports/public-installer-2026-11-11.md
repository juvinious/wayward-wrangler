# Public installer verification — 2026-11-11

Scope: remove mandatory GitHub authentication from installation, select latest
stable by default, retain exact-version selection and existing integrity/staging.
Hosted builder DNS, CLI update/uninstall and first publication/package channels
were explicitly deferred for separate review. No account, operator installation,
service replacement, remote write or publication occurred.

## Observed failure and implementation

Before changing scripts/install.sh, the exact-version regression failed with
`Unexpected authenticated GitHub CLI call` from the deliberately unusable gh
transport. Default installation failed with `An explicit release version is required`.
These were behavior failures, not missing dependencies or compilation failures.

The default installer now uses curl over HTTPS for public release metadata and
assets. Node, already required by the model runtime, parses metadata as data;
Python and jq are not installed runtime dependencies. The installer validates
requested/returned tags, published stable selection, required assets, GitHub's
SHA256SUMS asset digest, archive SHA256 and safe archive paths/types before using
the existing paired bundle validator/staged installer. Missing digest/asset data
is refused rather than weakening verification. Public curl ignores curlrc and
restricts redirects to HTTPS; returned metadata URLs are not executed or trusted.
The trust root remains the selected GitHub repository and HTTPS, without a new
independent signature/attestation service.

Node >=22.19, Git and flock are checked before download. Node is not provisioned.
`--private` explicitly retains gh-owned authentication for private releases; the
public path never falls back to it. Bash PATH hints support spaces/quotes. The
obsolete success instruction to upgrade each project has been removed.

## Evidence

Ten focused offline checks pass for public latest/exact selection, explicit
private access, unchanged-default installation, metadata/digest/checksum refusal,
traversal/symlink refusal and an unusable Node prerequisite. The public transport
fixture rejects gh access and authentication/insecure curl options.

The real packaged-install check passes using the actual archive, shell installer,
paired binaries and temporary HOME/XDG/project. It exercises piped shell input,
latest and exact installation, version/help/backend validation, all18 manifest
file hashes, damaged-resource/license refusal and installed service/IPC/TUI
startup/detach outside the source tree. Corrupt and unavailable archive downloads
leave the same working release selected with no staged install. Rust is absent
from runtime PATH; strace reports zero source-checkout accesses. No model request
or systemd bootstrap is involved; only HTTPS transport is a local fixture.

The locked remapped release, notice inventory and source/binary/full-bundle
privacy checks pass. Shell/Bash examples, maintained-document references and
diff checks pass. Gitleaks reports no leaks in this task's changed/new source.
No dependency, compiler or Rust application logic changed.

Validation archive: `waywr-v0.4.0-local.public-linux-x86_64-gnu.tar.gz`.
Archive SHA256: `8244de03ab75b3d8c0bd55832c87bf96347a97ee57cd93ffc4df0165dfd93f10`.
Manifest SHA256: `ded056d81257dcc29225888ecccfce9468d46ef856f9106f69a5d24bbd2005b0`.
It was assembled from the working tree at parent cafc725 before the local commit;
it is a validation artifact, not a published release.

## Remaining boundary

The updated installer and first release assets still need publication. No live
GitHub/CDN download or independent TLS transport test is claimed by these fixtures.
Update-source metadata, CLI update/uninstall, artifact cleanup and Node provisioning
remain separate work. See the [human guide](../distribution.md),
[implementation notes](../agents/distribution.md) and short [handoff](../HANDOFF.md).
