---
title: Self-Hosting the Backup Server
category: Reference
tags: [backup, server, docker, self-hosting, reference]
---

Granit's [[backups|cloud backups]] go to a backup server you run yourself. There is no
hosted service: the server is a small Docker Compose stack meant for a home server, a
NAS, or a cheap VPS, and it only ever stores ciphertext. This page covers what the
stack consists of, how to deploy it, how to manage API keys, and how to keep it
running.

# What runs where

The stack is four containers, with a single public entry point:

| Service | Image | Role |
|---|---|---|
| `caddy` | `caddy:2-alpine` | The only host-published service. Terminates HTTPS and routes requests to the two services below. |
| `backend` | `ghcr.io/lhelge/granit-server` | The REST API the app talks to: registers snapshots, hands out time-limited upload and download URLs, and checks API keys. |
| `postgres` | `postgres:17-alpine` | The snapshot catalog: which snapshots exist, their size, checksum, and state, and which API key owns them. |
| `rustfs` | `rustfs/rustfs` | S3-compatible object store holding the encrypted archives. The app uploads and downloads archives directly against it, through Caddy. |

The `backend` image is published to the GitHub Container Registry with every Granit
release, tagged with the release version and `latest`. The same `backend/` directory in
the repository also contains the `Dockerfile` if you prefer to build it from source.

# Prerequisites

- A host with Docker and the Compose plugin (`docker compose version` should work).
- For anything beyond a local trial, a DNS name pointing at the host, with ports 80 and
  443 reachable. Caddy then provisions and renews HTTPS certificates automatically.

Granit sends the API key as a bearer token on every request, so run the server behind
HTTPS whenever it is reachable from outside your own network.

# Deploying

## Get the deployment files

The deployment is two files, `docker-compose.yml` and `Caddyfile`, found in the
`backend/` directory of the [Granit repository](https://github.com/LHelge/granit).
Either clone the repository or copy just those two files into a directory on the host.

The repository's compose file builds the backend from source. To use the prebuilt image
instead, replace the `build:` block of the `backend` service with an `image:` line:

```yaml
  backend:
    image: ghcr.io/lhelge/granit-server:latest
    restart: unless-stopped
    # ... the rest of the service stays as it is
```

Pin a specific version (for example `ghcr.io/lhelge/granit-server:0.9.0`) if you would
rather upgrade deliberately.

## Configure

Compose reads its settings from a `.env` file next to `docker-compose.yml`. Every
variable has a default that suits a local trial only, so for a real deployment create
the file with at least these:

```sh
# Credentials for the two data stores. Pick long random values.
POSTGRES_PASSWORD=change-me
S3_ACCESS_KEY=change-me
S3_SECRET_KEY=change-me

# The origin the desktop app reaches the server on, and the Caddy site
# address. With a real domain Caddy serves HTTPS on it automatically.
PUBLIC_ORIGIN=https://backup.example.com
CADDY_SITE_ADDRESS=backup.example.com
```

Then, in `docker-compose.yml`, change the `caddy` service's published port from
`8080:8080` to `80:80` and `443:443` so Caddy can answer HTTP challenges and serve
HTTPS.

> [!IMPORTANT]
> `PUBLIC_ORIGIN` must be exactly the URL you enter as **Backend URL** in Granit. The
> upload and download URLs the server hands out are cryptographically signed for that
> origin, so a mismatch — a different scheme, host, or port — makes every backup fail
> with a signature or connection error even though the API itself responds. It is the
> first thing to check when uploads fail.

For a trial on the machine you are sitting at, skip the `.env` file entirely: the stack
listens on `http://localhost:8080` with the built-in defaults.

## Start it

```sh
docker compose up -d
```

On first start the backend runs its database migrations and creates the object-store
bucket by itself. It restarts a few times while RustFS is still coming up, which is
expected. Check that it is healthy:

```sh
docker compose logs backend
curl https://backup.example.com/healthz     # prints "ok"
```

# Managing API keys

Every request from Granit is authorised by an API key. Keys are created on the server,
and the token is printed exactly once — only a SHA-256 hash of it is stored, so a lost
token cannot be shown again, only replaced.

```sh
# Create a key. The name is just a label; naming it after the machine helps later.
docker compose exec backend granit-server create-key --name laptop

# See what exists, and revoke one by id.
docker compose exec backend granit-server list-keys
docker compose exec backend granit-server revoke-key --id <uuid>
```

Snapshots belong to the key that uploaded them. Listing, restoring, and deleting all
operate on that key's snapshots only, which has two practical consequences:

- **Keep the token with the passphrase.** A key is one of the three things needed for a
  [[backups#disaster-restore|disaster restore]], next to the backend URL and the
  encryption passphrase. A new key created after a machine is lost cannot see the old
  machine's snapshots. Store the token in the same password manager entry as the
  passphrase.
- **Revoking a key replaces its token.** The command invalidates the old token and
  prints a new one once, keeping the same ownership ID and access to all existing
  snapshots. Save the replacement and update Granit's settings. Previously issued
  upload and download URLs remain valid until their expiry, up to 15 minutes.

One key per machine keeps the blast radius small: a leaked token can only list, upload,
or delete that machine's snapshots, and it can never decrypt any of them.

# Connecting Granit

In **Settings → Backup** enter the backend URL — the `PUBLIC_ORIGIN` from above — and
the `grnt_…` token, then set an encryption passphrase.
[[backups#configuring-a-cave|Configuring a cave]] walks through the rest.

# Upgrading

New backend versions ship with each Granit release. Pull and restart:

```sh
docker compose pull
docker compose up -d
```

Database migrations run automatically on startup. Upgrading the desktop app and the
server together is the safe default, since both come from the same release.

# Backing up the server

All state lives in two Docker volumes: `pgdata` (the catalog) and `rustfs-data` (the
archives). Back them up together, since a catalog without its archives, or the other
way round, is useless. Snapshotting the host's disk or using `docker run --rm -v` with a
`tar` of each volume both work. The `caddy-data` volume only holds certificates and can
be regenerated.

# Security notes

- The server never sees plaintext. Archives are encrypted on your machine with a key
  derived from your passphrase, and the passphrase and key are never sent anywhere. See
  [[backups#key-file-warning|the key file warning]] for the one thing to be careful
  about on the client side.
- An API key authorises deletion, so treat tokens as secrets even though they cannot
  decrypt anything.
- Change the default postgres and object-store credentials before exposing the host to
  anything but yourself. The defaults exist to make a local trial a one-liner, nothing
  more.
