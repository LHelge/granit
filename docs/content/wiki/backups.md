---
title: Cloud Backups
category: Reference
tags: [backup, encryption, server, reference]
---

Granit can back up a whole cave to a self-hosted backup server. Each backup is a full
snapshot: the cave is packed into a compressed archive, encrypted on your machine, and
uploaded to the server's object store. All snapshots are kept, and the server only ever
sees ciphertext — the encryption passphrase and key never leave your machine.
Restoring a snapshot from the app is planned but not implemented yet; until then a
snapshot can be decrypted manually with the passphrase.

# How it works

When you press **Back up now** (in **Settings → Backup**), Granit:

1. Packs the cave — notes, attachments, and the `.granit/` configuration and templates —
   into a `tar` + `zstd` archive. Three things are excluded: the local key file
   (`.granit/backup.key`), the regenerable embeddings cache (`.granit/embeddings.bin`),
   and any `.git` directory.
2. Encrypts the archive with XChaCha20-Poly1305 using a key derived from your
   passphrase with Argon2id. The salt and derivation parameters are stored in the
   archive header, so a future restore needs only the passphrase and the archive —
   nothing from the original machine.
3. Registers the snapshot with the backup server and uploads the encrypted archive
   directly to its object store via a time-limited upload URL.

Existing snapshots are listed at the bottom of the Backup settings section, newest
first, with their size and state.

# Setting up a server

The server is part of the Granit repository (`backend/`) and runs as a small Docker
Compose stack: postgres for the snapshot catalog, an S3-compatible object store, the
`granit-server` API, and a Caddy reverse proxy as the single public entry point. See
`backend/README.md` in the repository for deployment instructions.

API keys are created on the server:

```sh
docker compose exec backend granit-server create-key --name laptop
```

The `grnt_…` token is printed exactly once — only a hash of it is stored.

# Configuring a cave

In **Settings → Backup**:

- **Backend URL** — the public origin of your server, e.g. `https://backup.example.com`.
- **API key** — the token created above.
- **Encryption passphrase** — set once per cave (minimum 8 characters). Pressing
  **Set** derives the encryption key and caches it in `.granit/backup.key`, so routine
  backups never prompt for the passphrase.

The URL and API key are saved with the other cave settings (see [[configuration]]) —
both **Save** and **Back up now** apply them. The passphrase itself is never written
anywhere.

> [!CAUTION]
> A forgotten passphrase makes your backups permanently unrecoverable. There is no
> reset or recovery mechanism — the server cannot decrypt anything. Store the
> passphrase somewhere safe, such as a password manager.

Changing the passphrase only affects future snapshots: older ones still require the
passphrase they were made with.

# The key file and cave syncing {#key-file-warning}

The derived encryption key is cached in `.granit/backup.key` inside the cave. It is
never included in backup archives and never sent to the server, and on its own machine
it adds little risk — anyone with access to the cave directory already has your notes
in plaintext.

> [!WARNING]
> The key file travels with the cave. If you sync or version the cave directory with
> another tool — a git repository pushed to a remote, Dropbox, iCloud Drive, or similar
> — `.granit/backup.key` is copied to that service **unencrypted**, and whoever can
> read it can decrypt all your backup snapshots. If your cave is a git repository, add
> `.granit/backup.key` to its `.gitignore`. The same caution applies to the provider
> API keys stored in `.granit/config.yml`.

Copying the cave to another machine yourself is fine — the key comes along and backups
keep working there without re-entering the passphrase.
