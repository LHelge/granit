---
title: Installation
category: Getting Started
tags: [install, download, updates]
---

Granit is distributed as prebuilt desktop bundles from the project's GitHub releases. This page covers where to download the app, how to install it on each supported operating system, and how automatic updates behave. Once installed, continue with [[getting-started]] to open your first cave.

# Downloading

All release artifacts are published on the [[download|releases page]] at <https://github.com/LHelge/granit/releases>. Each tagged release lists the available bundles for every platform. Pick the artifact that matches your operating system from the sections below.

# macOS

Download the `.dmg` image from the release assets. Open the downloaded file, then drag the Granit app into your `Applications` folder. Launch it from `Applications` or Spotlight.

# Linux

Three Linux artifacts are published with each release:

- **AppImage** — a self-contained, portable binary. Make it executable and run it directly:

  ```sh
  chmod +x Granit_*.AppImage
  ./Granit_*.AppImage
  ```

- **`.deb`** — for Debian, Ubuntu, and derivatives. Install it with your package manager, for example:

  ```sh
  sudo apt install ./granit_*.deb
  ```

- **`.rpm`** — for Fedora, RHEL, openSUSE, and derivatives. Install it with `dnf` or `rpm`, for example:

  ```sh
  sudo dnf install ./granit-*.rpm
  ```

# Automatic updates

On startup Granit silently checks the latest GitHub release, downloads any new version, installs it, and offers to restart. After an update it shows the release notes for the version you just received. A manual check is also available from the About dialog inside the app.

> [!IMPORTANT]
> On Linux, only the **AppImage** build self-updates. If you installed the `.deb` or `.rpm` package, you must update manually by downloading and installing the new package from the [[download|releases page]]. The macOS `.dmg` build updates automatically.

Automatic updates become visible only once a release is fully published, and development builds never run the startup check — the manual check in the About dialog still works in those builds.

# Next steps

With Granit installed, head to [[getting-started]] to open your first cave, then read [[cave-rules]] to understand how notes are named and identified.
