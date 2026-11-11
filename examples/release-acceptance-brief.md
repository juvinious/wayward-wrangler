# Workboard — release acceptance project

Build a useful local web application for managing work across projects. Deliver
running software through the normal Waywr proposal, approval, delegated work,
review, checks and candidate-delivery flow. A plan, UI mockup or successful model
reply alone is not delivery.

## Users and data

- Local login/logout with securely hashed passwords and sessions. Provide an
  explicit development seed command for synthetic demonstration users.
- Projects with owner, editor and viewer membership. Enforce project access and
  write permissions on the server, including direct API requests.
- Persistent tasks with title, description, status, priority, assignee, due date and labels.
  Include project task comments and an activity trail for task changes.
- Store data in SQLite with an explicit schema initialization/migration path.
  Restarting the server must preserve existing users, memberships and work.

## Everyday workflow

An owner creates a project, adds members and creates tasks. Editors update tasks
and comment; viewers inspect without changing data. The interface supports a
board or list, text search, status/assignee/label filters, task details and clear
empty/error states. Keep active filters in the URL so refresh retains the view.
Provide a CSV export of the filtered task list with correct CSV escaping.

The UI must work with keyboard navigation and at a narrow browser width. Use
clear labels and visible success/error feedback. A polished static mockup without
working authentication, API, permissions and persistence does not satisfy this brief.

## Delivery and verification

Choose a maintained, ordinary stack that runs locally. Explain the choice in the
proposal; do not require cloud services, production credentials or a hosted database.
Dependency installation is allowed as part of the approved project build. Keep a
lockfile and document runtime prerequisites.

Deliver:

- Connected UI/API/database implementation, not disconnected worker demonstrations.
- A finite `sh scripts/check.sh` command covering the meaningful automated checks,
  including server-side access denial, validation, filtering/export and persistence.
- A `sh scripts/build.sh` command for the complete application build.
- README instructions for dependency installation, development seed, checks, build,
  localhost startup, data location and shutdown. Preview servers bind to loopback.

Use synthetic records only. Keep provider credentials, session secrets and runtime
data outside Git; example configuration contains placeholders. Do not reuse data
or configuration from another project. No deployment, GitHub publication, external
OAuth/email, billing or global package installation is part of this task.

## Concrete acceptance example

1. Seed four synthetic users. An owner creates a project and grants editor/viewer
   roles. A fourth user outside the project cannot read its data through the API.
2. Create tasks with different statuses, priorities, assignees and labels. Add a
   comment, change a task's status and verify its visible activity trail.
3. Search and filter the list, refresh the browser, and export CSV. Exported rows
   must match the visible filter; commas, quotes and newlines in text stay valid.
4. Verify a viewer can inspect but cannot create/edit/delete tasks or add comments,
   even with direct API calls. Invalid task input produces an actionable error.
5. Restart the server and reopen the project. Data and memberships remain; logout
   invalidates the session.
6. From the delivered commit, follow the README on a fresh local data directory,
   run checks/build and inspect the working application in a browser.

Use the coordinator to choose a sensible task graph and delegate at least two
meaningful worker assignments. Review exact submitted revisions, address real
findings, integrate and check the combined result. Keep final delivery local to
this test repository's main branch. Human candidate review is required; do not
replace it with a model's claim of visual acceptance.
