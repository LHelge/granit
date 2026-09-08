---
title: Cloud Backups
category: Reference
tags: [backup, encryption, server, reference]
---

Granit can back up a whole cave to a self-hosted backup server. Each backup is a full
snapshot: the cave is packed into a compressed archive, encrypted on your machine, and
uploaded to the server's object store. The server only ever sees ciphertext — the
encryption passphrase and key never leave your machine. Any snapshot can be
[[#restoring-a-snapshot|restored]] from the app, into a new folder or over the current
cave, including onto a machine that has never seen the cave before. Snapshots are kept
until you [[#deleting-snapshots|delete them]].

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

The server is designed to be self-hosted: a small Docker Compose stack — postgres for
the snapshot catalog, an S3-compatible object store, the `granit-server` API, and a
Caddy reverse proxy as the single public entry point — that runs on a home server, a
NAS, or a VPS. [[backup-server]] covers deploying it, creating API keys, upgrading, and
what to back up on the server side.

# Configuring a cave {#configuring-a-cave}

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

Restoring validates the archive's key-derivation parameters before running
Argon2: at most 256 MiB of memory, 10 iterations, and 16 lanes. Current backups
use 64 MiB, 3 iterations, and 4 lanes. Unsupported parameters are rejected.

# Restoring a snapshot {#restoring-a-snapshot}

Every complete snapshot in the list (**Settings → Backup**) has a **Restore** action
with two targets:

- **Restore to new folder** — pick an empty (or new) folder; the snapshot is unpacked
  there and Granit switches to the restored cave. The original cave is untouched.
- **Replace current cave** — rolls the open cave back to the snapshot, after an
  explicit confirmation. The restore is downloaded, verified, and unpacked next to the
  cave first, then swapped into place, so a failure at any point — network, wrong
  passphrase, full disk — leaves the current cave exactly as it was.

After a roll-back the previous contents are kept beside the cave folder as
`<cave>.pre-restore-<timestamp>`. That directory is your escape hatch: nothing deletes
it automatically, so once you have confirmed the restore did what you wanted, delete it
yourself to reclaim the space.

During a restore Granit verifies the downloaded archive against the checksum recorded
at upload time, then decrypts it. If the cave's cached key matches the snapshot it is
used silently; otherwise — the snapshot was made under an older passphrase, or the key
file is gone — Granit asks for the passphrase the snapshot was encrypted with and
derives everything else from the archive itself.

## Restoring on a fresh machine {#disaster-restore}

If the machine is lost, everything needed for recovery is the backend URL, the API key,
and the passphrase. In Granit's no-cave state (before any cave is opened), the sidebar
footer offers **Restore a cave from backup…**: enter the backend URL and API key, list
that key's snapshots, pick one and an empty target folder, and enter the passphrase.
The restored cave opens when the restore finishes, and future backups from it prompt
for a passphrase once to rebuild the local key file.

# Deleting snapshots {#deleting-snapshots}

Each snapshot row has a **Delete** action that opens a confirmation panel below the
list. Deleting removes both the stored archive and the server's record; pending snapshots — uploads that never
completed — can be deleted too, which is how abandoned uploads are cleaned up.

Deletion is also the only way snapshots actually disappear after a passphrase change:
old snapshots encrypted under a retired passphrase are kept (and restorable with that
passphrase) until you delete them.

# The key file and cave syncing {#key-file-warning}

The derived encryption key is cached in `.granit/backup.key` inside the cave. It is
never included in backup archives and never sent to the server, and on its own machine
it adds little risk — anyone with access to the cave directory already has your notes
in plaintext.

On Unix, the key file is created with owner-only permissions before any key
bytes are written, including when replacing an existing cache.

> [!WARNING]
> The key file travels with the cave. If you sync or version the cave directory with
> another tool — a git repository pushed to a remote, Dropbox, iCloud Drive, or similar
> — `.granit/backup.key` is copied to that service **unencrypted**, and whoever can
> read it can decrypt all your backup snapshots. If your cave is a git repository, add
> `.granit/backup.key` to its `.gitignore`. The same caution applies to the provider
> API keys stored in `.granit/config.yml`.

Copying the cave to another machine yourself is fine — the key comes along and backups
keep working there without re-entering the passphrase.
