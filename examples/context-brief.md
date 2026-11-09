# Example Notes: manual export interaction

Write a short product interaction specification for the existing initial release.
Use the explicitly inherited approved product scope and accepted decisions.

Specify how a user exports all notes to a filename they choose in a local folder,
including the ordinary success path, an invalid destination, and observable
acceptance checks. This remains a text-only design exercise, not implementation.

Users may repeatedly select the same filename; existing backup files can already
occupy that destination. The current approved scope does not define how this
situation should behave. Avoid data-loss assumptions that the owner has not made.

Keep the proposal focused on manual export. Do not reopen the established backup
policy. Identify substantive unresolved product choices from the supplied evidence
and request an owner decision where required; leave affected behavior provisional.
