---
title: Installation
category: Getting Started
tags: [install, download, updates]
---

Granit is distributed as prebuilt desktop bundles from the project's GitHub releases. This page covers where to download the app, how to install it on each supported operating system, how to build it from source, and how automatic updates behave. Once installed, continue with [[getting-started]] to open your first cave.

# Downloading

All release artifacts are published on the [[download|releases page]] at <https://github.com/LHelge/granit/releases>. Each tagged release lists the available bundles for every platform. Pick the artifact that matches your operating system from the sections below, or use one of the per-platform install commands described under each section.

# macOS

Granit ships a signed and notarized `.dmg` for Apple Silicon (aarch64).

## Homebrew

The simplest way to install on macOS is the [Homebrew](https://brew.sh) cask:

```sh
brew install --cask lhelge/tap/granit
```

This taps `LHelge/homebrew-tap` and installs the latest signed, notarized build. The cask enables auto-updates, so after the first install the app keeps itself up to date — Homebrew is only needed for the initial install.

## Manual `.dmg`

Alternatively, download the `.dmg` image from the release assets. Open the downloaded file, then drag the Granit app into your `Applications` folder. Launch it from `Applications` or Spotlight.

# Linux

Granit ships an AppImage, a `.deb`, and an `.rpm` for x86_64 Linux.

## Install script

The quickest way to get started on x86_64 Linux is the install script:

```sh
curl -fsSL https://granit.lhelge.se/static/install.sh | sh
```

This downloads the latest AppImage, installs it to `~/.local/bin/granit`, and sets up desktop integration (an icon and a `.desktop` launcher entry) so Granit appears in your application menu. Re-running the script upgrades an existing install, though the AppImage also self-updates once installed. If `~/.local/bin` is not on your `PATH`, the script prints a hint for adding it.

## Manual packages

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

# Windows

Granit ships an NSIS `.exe` installer and an `.msi` package for x86_64 Windows. Download either one from the release assets and run it to install.

> [!NOTE]
> The Windows builds are not yet code-signed. The first time you run the installer, Windows SmartScreen may show a "Windows protected your PC" dialog. Choose **More info**, then **Run anyway** to continue.

# Automatic updates

On startup Granit silently checks the latest GitHub release, downloads any new version, installs it, and offers to restart. After an update it shows the release notes for the version you just received. A manual check is also available from the About dialog inside the app.

> [!IMPORTANT]
> On Linux, only the **AppImage** build self-updates. If you installed the `.deb` or `.rpm` package, you must update manually by downloading and installing the new package from the [[download|releases page]]. The macOS `.dmg` (and the Homebrew cask) and the Windows builds all update automatically.

Automatic updates become visible only once a release is fully published, and development builds never run the startup check — the manual check in the About dialog still works in those builds.

# Building from source

Granit can also be built from source. You will need:

- **Rust** (stable) with the `wasm32-unknown-unknown` target:

  ```sh
  rustup target add wasm32-unknown-unknown
  ```

- **Node.js 22** for the frontend build pipeline.
- **Tauri CLI** and **Trunk**:

  ```sh
  cargo install tauri-cli --locked
  cargo install trunk --locked
  ```

With the prerequisites in place, build a release bundle:

```sh
npm ci
npm run build
cargo tauri build
```

The resulting bundles land under `src-tauri/target/release/bundle/`. Granit is dual-licensed under MIT and Apache-2.0.

# Next steps

With Granit installed, head to [[getting-started]] to open your first cave, then read [[cave-rules]] to understand how notes are named and identified.
