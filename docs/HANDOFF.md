# Waywr handoff — portable paired-bundle idle service replacement

Branch: `plan/usability-and-installation`. Base: `e15acaa` (thinking selection and
display). Follow [AGENTS.md](../AGENTS.md), the [product plan](usability-plan.md)
and [distribution contract](distribution.md). Local commits only; no remote
writes, operator service changes, preserved-project edits, global installation
or provider/account calls were performed by the agent.

## Current outcome

The user's authorized `target/local-bin/waywr` symlink launches a paired bundle
directly from a temporary directory. The old idle-handover code required the new
bundle's parent directory to be named releases, so successive local builds failed
with "Service replacement requires an installed paired application". This was an
application assumption, not a missing installation by the user.

Handover now derives the old paired root from its exact live process command and
checks that /proc identifies the same real service executable, including the OS
marker for a deleted old file. It checks the complete project/state/key/release/
IPC/runner/frontend command, systemd main PID, socket peer PID and project
snapshot before the unchanged execution/quiescence fence can stop anything.
A temporary or differently located new bundle does not need an installed releases
catalog. The new pair still passes normal manifest/resource/hash validation.
Live TUIs, pending delivery, active/uncertain execution and unrelated identities
still preserve the old owner. Only an owned, positively idle service can hand over.
Project settings/history and legacy release records are not rewritten.

The model-independent shared runtime, complete model picker, Thinking selector,
draft/active model-level labels and explicit profile/context/policy grants remain
unchanged. Catalog thinking levels come from SDK metadata; real account login,
model quality and useful native delivery remain unverified. Public download/
update/package channels remain incomplete. Node >=22.19 and flock remain runtime
prerequisites; the model tool process is trusted same-UID code, not an OS sandbox.

## Evidence

The real systemd/paired-binary/TUI regression first reproduced the exact error
through a temporary local-bin symlink, without installing the bundles. It now
bootstraps, rejects replacement for live UI/pending input/uncertain launch evidence,
then hands over after detach with the old bundle physically removed. New PID and
owner generation restore the saved draft; settings remain byte-identical and no
model call occurs. The original installed-layout systemd variant also passes.
Both use temporary HOME/projects and self-cleaning uniquely keyed units; neither
is an operator-project trial or proof of active-model interruption/resumption.

The focused command-identity check accepts arbitrary paired roots and deleted old
executables while rejecting mismatched executable, project, state, IPC or release.
CI/release workflows include this focused check locally; hosted runs remain the
user's responsibility. The remapped locked release build and source/binary/bundle
public-content checks pass. Cargo/npm locks and dependencies are unchanged.

## Next authorized action

The ignored local test symlink points to `v0.4.0-local.handover`, manifest SHA256
`7acbed9ffaec420cfb900178a9c32af65715c4e76a29e98aba2967518a5c7d50`.
The user detaches any currently open TUI and retries the same local command on
their project. The agent has not stopped that project's service or edited its
state. Any actual busy/uncertain ownership refusal still needs its specific
reconciliation; there is no force-stop or inferred settlement.

Owners: [distribution](distribution.md), [verification](verify-usability.md),
[providers](provider-selection.md), [controls](tui-controls.md),
[runtime direction](execution-adapter-decision.md).
