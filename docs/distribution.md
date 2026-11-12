# Installing and distributing Waywr

Waywr is installed as one application containing the terminal UI, project service,
worker runner and shared JavaScript model runtime. The installer puts these files
in your user account and creates a stable `waywr` command. Project files and
conversation state stay in each project's `.waywr` directory.

## What works today

| Method or command | Availability |
| --- | --- |
| Build and install this checkout | Available: `sh scripts/dev-install.sh` |
| Public release installer | Implemented over HTTPS without GitHub login; published installer and binary release assets are still needed |
| Latest stable or an exact version | Supported by `scripts/install.sh`; omit `--version` for latest stable |
| `waywr update` and `waywr uninstall` | Planned |
| Debian, npm or Homebrew packages | Not provided |

## Install from source now

Install the [source-build prerequisites](../README.md#build-and-install-from-source),
then run these commands from the Waywr checkout. Examples use Bash:

```bash
sh scripts/dev-install.sh
export PATH="$HOME/.local/share/wayward-wrangler/bin:$PATH"
waywr --version
```

The PATH change applies to this terminal. Add the same `export PATH` line to
`~/.bashrc` to keep it for future Bash sessions. Fish users can instead run
`fish_add_path "$HOME/.local/share/wayward-wrangler/bin"`.

The script compiles the application, builds the model runtime, checks the result
and installs all components together. It prints the installed version and launcher
path. No sudo is needed. A failed build leaves the previous installation selected.

To use it in another project:

```bash
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

1. A maintainer prepares a matching application version and dated
   [changelog entry](../CHANGELOG.md), reviews the commit through main, then pushes
   a version tag such as `v1.2.3` (illustrative).
2. GitHub Actions validates the tag/version/notes, pins the build and security audit
   to the same commit, and creates the Linux archive plus `SHA256SUMS`.
3. After all checks pass, the publish job verifies the tag still names that commit
   and creates a GitHub Release using the matching changelog notes. Preview tags
   are prereleases and do not replace latest stable.
4. The installer downloads the archive and checksums, verifies them, then installs
   the complete application and selects it for subsequent launches.

The [release guide](releases.md) covers cadence, preparation and exact tag commands.

An Actions build artifact alone is not a published release. The downloadable files
must exist on the GitHub Release before the release installer can work.

## Install a public binary release

The installer downloads public releases over HTTPS. It does not require GitHub
CLI, a GitHub account, Rust, Python or npm. The updated installer and release
assets must be published before the following download command can be used.
Replace `OWNER/REPO` with the release repository:

```bash
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/scripts/install.sh | sh -s -- --repo OWNER/REPO
```

This selects the repository's latest published stable release. To install an exact
version from a saved installer instead:

```bash
sh install.sh --repo OWNER/REPO --version v1.2.3
```

Use an actually published version in place of `v1.2.3`. `--prefix /absolute/path`
changes the application directory; `--no-select-default` installs a release
without switching the current application. `WAYWR_REPOSITORY` can supply the
repository instead of `--repo`. Private release access is an explicit option,
`--private`, which uses GitHub CLI's saved login; the default public path never
uses it or silently falls back to it.

Before downloading, the installer checks Linux x86_64, glibc2.39+, Node >=22.19,
Git and util-linux `flock`. Install those prerequisites first; Node is not bundled
or automatically installed. The download also needs curl, GNU tar, gzip and
sha256sum. See [Linux requirements](linux-sandbox-setup.md).

The installer verifies GitHub's SHA256SUMS asset digest, the archive checksum,
safe archive contents and the complete paired bundle. A failed download or
validation leaves the working version selected. After installation it prints
the stable launcher path and a Bash PATH command when needed. Open your project
and run `waywr`; no per-project application upgrade is required.

## Updates

Today, updating means rebuilding from source or installing another binary
release through the public installer. The planned command is:

```bash
waywr update
```

It will fetch the latest stable release from the saved repository, check it and
select it for new launches. Projects do not need to be rebound to that version.
Compatible work already running stays with its existing service until settled;
the UI can reattach to it. An idle service can switch when the project is reopened.
Updating must not silently stop or repeat a model call.

## Uninstall

There is no supported uninstall command yet. The planned command is:

```bash
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
