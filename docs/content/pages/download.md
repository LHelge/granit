---
title: Download
order: 1
---

Grab the newest build from the [latest release on GitHub](https://github.com/LHelge/granit/releases/latest), or use one of the quick install commands below.

# Quick install

- **macOS (Apple Silicon)** — install with [Homebrew](https://brew.sh):

  ```sh
  brew install --cask lhelge/tap/granit
  ```

- **Linux (x86_64)** — run the install script:

  ```sh
  curl -fsSL https://granit.lhelge.se/static/install.sh | sh
  ```

# Pick your platform

| Platform | Artifact |
|----------|----------|
| macOS (Apple Silicon) | `.dmg` |
| Linux | `.AppImage`, `.deb`, or `.rpm` |
| Windows | `.exe` or `.msi` |

Download the artifact for your system, then follow the full [[installation]] guide for setup and platform notes.

# Automatic updates

Granit checks GitHub releases on startup and can download and install new versions for you, then offer a restart and show the release notes.

Self-update support depends on how you installed it:

- **macOS (`.dmg`)**, **Linux `.AppImage`**, and **Windows** builds update themselves automatically.
- **Linux `.deb`** and **`.rpm`** installs do *not* self-update — re-download the latest artifact to upgrade.

A manual update check also lives in the app's About dialog.
