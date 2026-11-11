# Ordinary-project closeout verification (2026-10-18)

Implemented on historical branch `impl/ordinary-project-workflow`, including
`b810bf1` and all recovery/convergence ancestry; squash-merged through PR #10 as
`4218347`. [Cleanup](repository-cleanup-2026-10-18.md) preserves the original tips
under archive refs. Final production source: `258d8a89a737aaabff16cf28288927f4b6833370`,
clean when built with permanent Rust1.99.0, Linux x86_64/glibc2.39.
Release ID: `v0.0.0-ordinary.20261018.258d8a89a737` (local build identity; no tag/release published).
This supersedes `ordinary-20261018-e4354d3d2016` and interim closeout builds at
`981321a`/`f883d5d`/`8b6ece7`. The [initial evidence](../archive/product-workflow/2026-10-18-initial-verification-e4354d3.md)
is retained separately; its broad focused checks were reused, not repeated.

Bundle directory: `target/dist/v0.0.0-ordinary.20261018.258d8a89a737/waywr-v0.0.0-ordinary.20261018.258d8a89a737-linux-x86_64-gnu/`.
Archive: same parent/name with `.tar.gz`; SHA256SUMS is beside it.

| Artifact | SHA256 |
| --- | --- |
| manifest.json | `9dfb2c8735d1eeea65c400fd5cccef3225d80fe5739f8bd700b0fb8532d92465` |
| .tar.gz archive | `2ee32fe7ec4e0016f9a30527b01c41ab09aa7bd4db8de5b7a4531041976b23f5` |
| bin/waywr | `96ed8d71f3940691eac79cbebdcb0743764df1cb5724a5c656b1c8ff0f7cb044` |
| bin/waywr-runner | `5e963ea6ac24f7db5728954a700a37efaee0d3f4383d9c9d0055ced5ee77a494` |
| bin/waywr-service | `8353158dba5dc0cd670f5ce428bd0b92323d31475afa5fd7a2a07e42e39f5206` |

## Actual packaged installation

`scripts/test-packaged-install.py ARCHIVE` passed with this production archive.
The real shell installer downloaded fixture-transported archive/checksum bytes,
validated extraction, and invoked the real staged bundle installer. All 13 manifest
hashes matched. The normal user-local launcher resolved to the installed paired
binaries/resources. A corrupted resource was refused before installation.

The installed CLI, actual idle service and production TUI started and detached in
an isolated HOME/XDG prefix, outside the checkout, with Rust absent from PATH.
Strace observed zero source-checkout file accesses. The actual handshake separately
confirmed IPC1.10 and persistent state9; a stale assertion and mistaken state10
advertisement were corrected. All fixture-owned processes and temporary state were
removed. Operator PATH/installations/projects/services/bindings were unchanged.

Only gh's release transport was a fixture; no Waywr executable was replaced by a
stub. This does not verify authenticated release download, systemd bootstrap,
project upgrade, broad portability or an ordinary development job. Python/strace
are verification tools, not installed application runtime requirements.

## Ollama HTTP lifecycle and exact evidence

Existing shared evidence was inspected first: `ollama_metadata_http_does_not_generate_or_redirect`
exercised real metadata HTTP; `ollama_stream_public_only_exact_terminal_and_unknown_loss`
was decoder-only. Generic host cancellation/receipt tests did not establish HTTP
chat execution. The new real paired-runner HTTP fixtures now pass five cases:

- Cancellation before dispatch/send, with positive nonlaunch proof and zero chat POSTs.
- Cancellation after send: settled client, no action adopted, remote completion unknown.
- Terminal response versus cancellation: one effective outcome at the receipt gate.
- Saved terminal receipt reload after an owner-generation change; late cancellation
  cannot override it, and no second HTTP dispatch occurs.
- Ambiguous response loss: settled client with unknown remote outcome; replacement
  and false nonlaunch retirement are refused, with exactly one chat POST.

The shared real-Git/context HTTP fixture also passes coordinator and reviewer
retrieval/continuation using exact frozen commits, schemas and application-mediated
pages after the mutable checkout changes. Unresolved findings remain. It reuses the
existing context machinery; prior pagination/completeness checks are retained in
the initial evidence. No model, download or Ollama service was used. These tests
establish transport/state behavior, not Qwen quality or native autonomous recovery.
A lone saved decoded result without a validated receipt remains uncertain.

## Generated resources and focused checks

The representative existing retirement fixture now passes two successive output
inventories and an authorized rebuild. Exact approved `generated_outputs` roots,
matching prior hashes/ancestry and terminal hashes allow known ignored products to
retire. No matching saved output authority means no deletion. Preexisting, changed,
later unknown and nonignored files remain; clean owned worktrees/branches retire
once with exact commits retained under archival refs. No operator cleanup occurred.
Output declarations default empty and never amend old approvals. Unclassified
products still retain their checkout; this is intentionally conservative.

Four ordinary TUI checks, four existing adapter checks, the five HTTP runner cases,
exact-evidence continuation, the changed retirement fixture, all three offline
archive-refusal checks, IPC source/schema consistency, actionlint1.7.12, shell
syntax, maintained-document links and branch diff checks passed. The full prior
Git/context/candidate/recovery/upgrade fixture evidence is reused. Cargo.lock adds
48 required identities for reqwest0.13.5, removing/upgrading no existing identity.
Changed paths exclude credentials, runtime state, raw transcripts and build products.

## GitHub and remaining boundary

PR #10 is merged. The
final PR-head run
passed the complete workflow at `ee4d9b2` before the squash.
Initial hosted run
passed all focused tests, then packaging hit Git's container checkout ownership guard.
Both workflows now trust only `$GITHUB_WORKSPACE` in the ephemeral builder's Git
configuration; no wildcard trust, repository rule or check is bypassed. Corrected hosted run
passed the complete job at `258d8a8`: 27 Rust tests (13 ordinary backend,
4 adapter, 1 exact-evidence HTTP, 5 HTTP lifecycle, 4 ordinary TUI), the IPC source
check and three installer refusal tests; production bundle build; real packaged
installation/idle TUI check; and Actions artifact upload. The hosted package check
also recorded 13 matching manifest files, Rust absent from PATH, zero traced
source-checkout accesses and zero native calls. The local archive
was rebuilt and its real isolated installation rechecked from the corrected source;
the identity above is that actual local archive, separate from hosted CI artifacts.
The version-tag release workflow and authenticated release download/publication remain
unexecuted. Actions artifacts are CI evidence, not published release assets. The user performed the squash merge; no release tag or publication occurred.
Native model suitability, complex-project usability, autonomous reliability,
operator upgrades, independent bit-identical builds and broad Linux portability
remain unverified. Gemini/OpenCode execution and Ollama coding workers are unsupported.
