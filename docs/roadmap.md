# Product roadmap

Waywr's goal is a useful daily loop: open a project, give a development request,
review the result, leave, update the application and continue the same work.
The priorities are broad model support, a fluid application lifecycle, then
coordination proven through a substantial real application.

## Shared model support

The shared provider runtime and searchable provider/model/thinking selection are
implemented. Provider IDs, catalogs and native options belong in that layer,
not in coordinator branches. New models on supported protocols should be added
through metadata/configuration. New protocols or authentication methods can need
integration code. Catalog coverage does not imply account access or equal model
quality. See [providers](provider-selection.md) and the
[architecture decision](execution-adapter-decision.md).

## Installation and daily use

Local source installation, compatible project reopening, idle service handover,
busy-owner attachment and obsolete-session renewal are implemented. The TUI has
separate interrupt, leave and pause actions, live steering, editable follow-ups,
model controls and mouse-wheel scrolling. See [controls](tui-controls.md) and the
[daily-use walkthrough](verify-usability.md).

The [public installer](distribution.md#install-a-public-binary-release) now supports
latest-stable/exact-version HTTPS downloads without GitHub login. Publication of
the updated installer and first binary release remains. Update and uninstall must use the same
application bundle, preserve project data and require no per-project application
upgrade. Debian packaging follows a working install/update/uninstall route;
additional channels come later. More platforms require their own release builds.

## Coordination through a substantial application

Useful task delivery across selected models remains to be established. Before the
first release, a significant app must go through proposal, human scope approval,
coordinator delegation, worker implementation, exact review/checks and accepted
local delivery without manual repair of the orchestration.

The proposed [Workboard acceptance brief](../examples/release-acceptance-brief.md)
covers a local web app with login/roles, persistent projects/tasks/comments,
search/filtering, CSV export and a documented run path. It requires useful
connected software, not a model reply or UI mockup. Normal review/rework is part
of the journey; provider quality and application correctness are judged from
the delivered work and evidence. The run has not yet been performed.
