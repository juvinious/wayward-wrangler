# Documentation

## Using Waywr

| Guide | What it explains |
| --- | --- |
| [Getting started](../README.md) | What Waywr does, setup and everyday commands |
| [Installation and distribution](distribution.md) | Available install methods, release downloads, files, updates and uninstall |
| [Linux requirements](linux-sandbox-setup.md) | Runtime prerequisites and the execution trust boundary |
| [Providers and models](provider-selection.md) | Sign-in, model/thinking selection, custom providers and usage |
| [TUI controls](tui-controls.md) | Keyboard, mouse, editing, interrupt, leave and pause |
| [Daily-use walkthrough](verify-usability.md) | Work on a project, leave, rebuild and return |
| [Project layout](project-layout.md) | Project data, worktrees and supported migration |
| [Roadmap](roadmap.md) | Product direction and remaining work |
| [Security policy](../SECURITY.md) | Private reporting, privacy and trust boundaries |

## Understanding the implementation

| Reference | Subject |
| --- | --- |
| [Architecture](architecture.md) | Components, ownership and runtime boundaries |
| [Provider architecture decision](execution-adapter-decision.md) | Why the shared model layer uses independent Pi libraries |
| [Ordinary project workflow](ordinary-project-workflow.md) | Setup, source selection, candidates, checks and completion |
| [Conversation](conversational-coordination.md) | Request routing, questions, exact approvals and steering |
| [Live input](live-input.md) | Steering/follow-up persistence, editing and acknowledgement |
| [Context](context-delivery.md) | Compact working context and exact evidence retrieval |
| [Recovery](coordinator-recovery.md) | Recovery causes, findings and continuation |
| [State/history](contracts/state-contract.md) | Durable records and compatibility |
| [Execution](contracts/execution-contract.md) | Host ownership, receipts and result adoption |
| [IPC](contracts/session-protocol.md) | Commands, capabilities and inspection |
| [TUI contract](contracts/tui-contract.md) | Detailed presentation and input semantics |

## Agent guidance and reports

[Agent documentation](agents/README.md) contains implementation guidance and
selective task notes. [AGENTS.md](../AGENTS.md) is the working policy;
[HANDOFF](HANDOFF.md) is the short current checkpoint. These are separate from
the user guides above.

[Reports](reports/README.md) contain dated findings and verification evidence.
[Archives](archive/README.md) retain superseded plans, older contracts and historical
reports. Neither is a default startup reading list or a source of current permission.
