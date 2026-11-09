# Example Notes: launch proposal

Draft a short product proposal for a local desktop notes app, not implementation.
Audience: people who keep personal notes offline. The release checklist includes
both of these stakeholder requirements, which have not been reconciled:

1. Offline-only: note content never leaves the user's device.
2. Automatic nightly cloud backup of all notes, enabled by default for convenience.

The initial draft must retain and label the conflicting requirements as unresolved,
so an independent critic can identify the conflict. Do not invent user consent or
silently pick a policy. Compare a strictly local release with manual export against
an explicitly opt-in cloud-backup release. After the operator chooses, revise the
proposal consistently and include two observable acceptance checks. No code,
network calls, repository operations, or access to actual note data is needed.
