# Waywr handoff — in-app provider sign-in

Branch: `plan/usability-and-installation`. Base: `2c6451a` (live input/editor).
Current slice adds provider sign-in from operator feedback. Follow
[AGENTS.md](../AGENTS.md). Local commits only; the user handles remote delivery.
No operator project/service/binding/provider configuration or credentials were
changed. No global installation, account consent, browser launch or real inference
was performed by the agent. Isolated tests use dummy credentials and temporary HOME.

## Implemented outcome

The operator reports responsiveness and no locking, then encountered missing
Anthropic credentials. Authentication belongs to a provider/account, not each
individual model. /login now opens an SDK provider/method chooser inside Waywr;
F3 Profiles G starts with the selected profile's provider. Opening the picker
makes no authentication request. Explicit method selection starts one local SDK
child while the UI event loop and project service keep running. Provider names,
methods and labels come from SDK metadata; no model/provider allowlist was added.
Providers without interactive methods explain their environment/configuration route.

The dialog handles SDK choices, browser/device URLs, masked API-key/code entry,
callback completion and manual redirect fallback. The normal private editor
supports cursor movement/paste without composer history. Browser handoff uses
xdg-open with one HTTP(S) argument and no shell; Ctrl-B retries opening. Esc backs
out/cancels to the parent view; Ctrl-Q leaves immediately and drops only the
sign-in child. Draft, service work, model/scope choices and held inputs remain.
Successful sign-in does not resubmit messages or release queues.

Authentication uses ephemeral stdin/stdout pipes outside project IPC/outboxes,
transcripts and saved UI state. The SDK writes its existing private credential
store with mode0600, locking and atomic replacement. Raw exchange errors are not
forwarded because SDK errors may include response bodies. The execution runtime
also sanitizes credential-refresh failures before project diagnostics; a fake-fetch
regression reproduced response-body disclosure and now passes. The user's credentials
were not inspected. SDK OAuth prompt cancellation clears only its matching input.

The bundled runtime previously could not resolve the SDK's variable OAuth imports.
Its own complete static OAuth registration helper now embeds those flows for Node;
no Bun runtime, provider framework, dependency or lock change was added. The CLI
fallback now displays SDK method choices correctly. Missing-credential replies
point to /login. Duplicate “Reply unavailable” status/heading text renders once,
including retained messages, without changing reason or recovery semantics.

## Evidence and limits

Behavioral red preceded implementation: Settings offered no sign-in, the runtime
never offered an interactive pipe prompt, packaged OAuth failed before its first
choice, and failure headings repeated. Focused regressions now pass. The actual
TUI/service/SDK fixture saves a dummy key privately, masks entry, cancels a second
input, retains the unsent composer draft, leaves promptly, finds no credential
text in project files/history/database and creates zero conversation turns.
It uses a directly launched isolated service, not an operator/systemd mutation.

Runtime checks cover real SDK API-key save/cancel and bundled Anthropic OAuth's
initial choice/cancellation before any browser/account request. A narrow SDK
interaction fixture covers browser URL/manual callback cancellation and raw-error
suppression; it is not actual OAuth success. Shared SDK worker/tool/steering
checks on two local HTTP protocols still pass. UI tests cover private editing,
method choice, browser intent, callback return, duplicate key presses and drafts;
browser command checks cover argument/protocol handling. Release build and
source/binary/bundle public-content checks pass; CI/release checks are prepared
locally, not run remotely.

Actual desktop browser opening, consent/token exchange, account access/refresh,
model quality and useful task delivery remain operator verification. A saved API
key is not proof of account/model access. Public installer/update/uninstall,
workflow coordinator usage summaries and the substantial application remain later
work. Do not turn fixture evidence into authorization for account calls.

## Existing lifecycle and next action

`2c6451a` implements Enter steering at the exact active attempt's next SDK boundary,
Alt+Enter separate durable follow-ups, F7/E pending-input restoration, contextual
multiline/history, direct model/thinking/status controls, interrupt/leave/pause
separation and no replay after interruption. Earlier commits renew obsolete or
stranded sessions after exact-owner fencing while retaining source/Git/worktrees;
compatible busy owners remain attachable across builds. Those contracts remain.

The ignored target/local-bin/waywr is the paired temporary local launcher;
target/local-bin/candidate.json records its actual source/artifact identity.
No global installation. Next is the bounded [operator walkthrough](verify-usability.md):
relaunch that launcher, use /login, then deliberately send useful work. Continue
from concrete friction; no broad test campaign or orchestration redesign first.

Owners: [providers](provider-selection.md), [controls](tui-controls.md),
[live input](live-input.md), [architecture](architecture.md), [plan](usability-plan.md).
