# Waywr handoff — retired settings no longer block startup

Branch: `plan/usability-and-installation`. Base checkpoint: `9c016a7` (shared
model runtime); this checkpoint fixes the first ordinary-use startup failure.
Follow [AGENTS.md](../AGENTS.md), the [product plan](usability-plan.md) and
[provider contract](provider-selection.md). Local commits only; the user handles
remote delivery. No operator installation, project mutation or account/model call
was performed by the agent.

## Current outcome

An existing state9 project with schema2 native preferences previously killed the
service before readiness, hiding the configuration error behind the launcher's
timeout. The service now reads those settings as inspection metadata, opens the
project/history and exposes F3 while execution stays blocked. No native execution
route was restored. Schema3 remains mandatory for saving and model routing.

F3 offers a fresh unsaved shared-runtime draft, without translating native choices.
Choose provider/model, S save, then section4 Switch: G explicitly grants selected
project context, Enter requests the switch. Saving alone leaves the old active
binding blocked. Existing request policy amendments and uncertain execution
settlement remain explicit. Historical preference revisions/binding hashes remain
unchanged. Old discovery caches refresh through the shared runtime, never through
retired native executables. Readiness shows the concrete configuration cause.

The ignored `target/local-bin/waywr` symlink is the user's requested local test
entry. It points to a temporary paired bundle, not a global installation. The
bundle identifies itself as `v0.4.0-local.setup-fix`; its manifest SHA256 is
`8d753a344db1daccee4fe86f903d82015b3c6518c74588a31eacd8e813372f0b`.
Source, all three remapped binaries and all19 bundle artifacts pass the
public-content check. The failed operator unit was inspected read-only and is
unloaded; the agent did not
restart it or edit the preserved project's settings.

## Verification and limits

The backend regression first failed on the actual schema2 startup rejection. It
now opens/reopens, preserves the old configuration and historical revision,
blocks execution, saves a schema3 draft without changing the binding, refuses a
switch without a context grant and accepts an explicitly granted switch. F3's
regression first failed because the form retained schema2; it now presents and
saves the new draft without an implicit command on opening. Existing shared
selection/frozen-destination and F3 controls remain checked.

A remapped locked release build passes. The packaged launcher/TUI/service check
uses a temporary project/HOME and real uniquely keyed systemd units with schema2
settings. Bootstrap, F3 setup, detach/reopen and idle handover after removing the
old build pass, preserving settings/draft and uncertain launch evidence. Test
units clean up; no model calls occur. This establishes startup/setup access, not
real provider authentication or useful model delivery.
The updated direct-service CI reopen path also passes with schema2 settings,
preserving its saved pause and draft. Workflow edits remain local, not hosted CI
evidence.

The shared runtime still uses pinned standalone pi-ai/pi-agent-core libraries,
not Pi's app: broad provider/model catalogs and custom protocols/options remain
outside coordinator code. Earlier two-protocol SDK/tool/runner fixture evidence
is unchanged. Node >=22.19 and util-linux flock remain runtime prerequisites.
The tool process is trusted same-UID code, not an OS sandbox. Public download,
update/uninstall and additional package channels remain incomplete. Hosted
security/build changes have not been run by the agent on GitHub; no paid features
were enabled. Source and bundle privacy checks remain required for delivery.

## Next authorized action

The user retries the local test entry on the existing project and explicitly
chooses the shared provider/model and coordinator switch. Authentication and any
actual model call remain the user's next ordinary-use step, not agent authority.
Fix the next concrete interruption in that journey; do not add provider-specific
coordinator branches or broaden testing into another acceptance campaign.

Owners: [providers](provider-selection.md), [runtime direction](execution-adapter-decision.md),
[distribution](distribution.md), [verification](verify-usability.md),
[architecture](architecture.md). Prior native evidence is [archived](archive/README.md).
