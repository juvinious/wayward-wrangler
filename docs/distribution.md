# Installing and distributing Waywr

Waywr is installed as one application containing the terminal UI, project service,
worker runner and shared JavaScript model runtime. The installer puts these files
in your user account and creates a stable `waywr` command. Project files and
conversation state stay in each project's `.waywr` directory.

## What works today

| Method or command | Availability |
| --- | --- |
| Build and install this checkout | Available: `sh scripts/dev-install.sh` |
| Download a published binary release | Installer exists, but requires GitHub CLI login, a repository and an exact version; no published binary release is recorded yet |
| Public curl installer with latest-version selection | Planned |
| `waywr update` and `waywr uninstall` | Planned |
| Debian, npm or Homebrew packages | Not provided |

## Install from source now

Install the [source-build prerequisites](../README.md#build-and-install-from-source),
then run these commands from the Waywr checkout. Examples use Fish:

```fish
sh scripts/dev-install.sh
fish_add_path "$HOME/.local/share/wayward-wrangler/bin"
waywr --version
```

The script compiles the application, builds the model runtime, checks the result
and installs all components together. It prints the installed version and launcher
path. No sudo is needed. A failed build leaves the previous installation selected.

To use it in another project:

```fish
cd PROJECT
waywr
```

Waywr prompts for setup when needed. Use `/login` to configure a provider, then
choose a model in F3. Installing the application does not sign in to a provider.
See [provider setup](provider-selection.md).

To update from source, run `sh scripts/dev-install.sh` again after changing the
checkout, then reopen the project. No per-project application upgrade is needed.

## How binary releases reach users

The release workflow is configured to perform this sequence:

1. A maintainer publishes a version tag such as `v1.2.3` after reviewing the code
   and passing the required checks. This version is illustrative.
2. GitHub Actions compiles the Linux application and model runtime, checks the
   bundle, and creates `waywr-v1.2.3-linux-x86_64-gnu.tar.gz` plus `SHA256SUMS`.
3. The publish job attaches those files to a GitHub Release for the tag.
4. The installer downloads the archive and checksums, verifies them, then installs
   the complete application and selects it for subsequent launches.

An Actions build artifact alone is not a published release. The downloadable files
must exist on the GitHub Release before the release installer can work.

The current installer takes `--repo OWNER/REPO` and `--version VERSION`, downloads
through authenticated GitHub CLI, and installs without compiling on the user's
machine. [Versioned binary installation](../README.md#versioned-binary-installation)
shows its current commands. It does not yet provide the public installation flow
below.

## Planned public installation

The target is a single command that downloads the installer from the release
repository and installs the latest stable release without a GitHub account or
source checkout. This example describes the planned interface; it does not work
with today's installer. Replace `OWNER/REPO` with the release repository:

```fish
curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/scripts/install.sh | sh -s -- --repo OWNER/REPO
```

The installer must choose the correct supported archive, verify the download and
all bundled files, then make the stable `waywr` command available. Users who need
a particular release can pass `--version v1.2.3`; `--prefix` chooses an alternative
installation directory. The repository is saved at installation so updates do not
require it again. A failed download or validation must leave the working version
selected. A first install must print the exact PATH command if one is needed.

The initial target is Linux x86_64 with glibc2.39 or newer, a systemd user manager,
Git, `/bin/sh`, Node >=22.19 and util-linux `flock`. Binary users do not need Rust,
Python or npm. Node is currently an external prerequisite; the public installer
must check it before reporting success. Automatic installation of a private Node
runtime is not implemented. Other platforms need separate release builds.
See [Linux requirements](linux-sandbox-setup.md) for the execution trust boundary.

## Updates

Today, updating means rebuilding from source or installing another exact binary
release through the current installer. The planned command is:

```fish
waywr update
```

It will fetch the latest stable release from the saved repository, check it and
select it for new launches. Projects do not need to be rebound to that version.
Compatible work already running stays with its existing service until settled;
the UI can reattach to it. An idle service can switch when the project is reopened.
Updating must not silently stop or repeat a model call.

## Uninstall

There is no supported uninstall command yet. The planned command is:

```fish
waywr uninstall
```

It will remove Waywr-owned launchers and application files. If work is still
running, it must explain what is active and let the user settle or deliberately
stop it before removing the files it needs. Project source, Git history, `.waywr`
state and provider credentials are preserved by default.

## Where files go

These are the default locations; `--prefix` changes the application directory.

| Location | Contents |
| --- | --- |
| `~/.local/share/wayward-wrangler/releases/RELEASE_ID/` | UI, service, runner, model runtime, license notices and file manifest |
| `~/.local/share/wayward-wrangler/current` | Link selecting the application for new launches |
| `~/.local/share/wayward-wrangler/bin/waywr` | Stable launcher; the default install also creates `~/.local/bin/waywr` |
| `PROJECT/.waywr/` | Project settings, conversation, worktrees and execution history |

Older application versions are retained today. Automatic cleanup is still planned.
Package-manager channels will follow the public installer and reuse the same
release contents. They must preserve the same project data and update behavior.
