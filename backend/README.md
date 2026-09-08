# granit-server — cave backup backend

A small self-hosted service that stores encrypted cave backups from the
Granit desktop app. Four containers: postgres (snapshot catalog), RustFS
(S3-compatible object store), the axum backend, and Caddy as the single
public entry point.

Granit encrypts every archive locally with a key derived from a passphrase
that never leaves the machine — the server only ever sees ciphertext.

## Quick start

```sh
cd backend
docker compose up --build -d

# Mint an API key (the token is printed exactly once):
docker compose exec backend granit-server create-key --name laptop
```

In Granit, open **Settings → Backup** and enter:

- **Backend URL**: `http://localhost:8080` (the Caddy origin)
- **API key**: the `grnt_…` token printed above

## Prebuilt image

Every release also publishes the backend as a container image, so a
deployment need not build from source:

```
ghcr.io/lhelge/granit-server:<version>   # e.g. 0.8.0
ghcr.io/lhelge/granit-server:latest
```

Only the backend is published; postgres, RustFS, and Caddy come from their
upstream images as in `docker-compose.yml`. To use the prebuilt image with
compose, replace the `build:` block of the `backend` service with
`image: ghcr.io/lhelge/granit-server:latest`.

## Key management

```sh
docker compose exec backend granit-server list-keys
docker compose exec backend granit-server revoke-key --id <uuid>
```

Only SHA-256 hashes of keys are stored; a lost token cannot be recovered,
only replaced.

`revoke-key` invalidates the current token and prints a newly generated
replacement once. Update the API key in each desktop using that credential.
The key's ID and name remain the same, so all existing snapshots and pending
uploads remain accessible with the replacement. Previously issued presigned
URLs remain valid for up to 15 minutes. Already-revoked legacy keys are not
reactivated by this command.

## Environment variables

Compose reads these from the shell or a `.env` file next to
`docker-compose.yml`; defaults in parentheses suit local use only.

| Variable | Meaning |
|---|---|
| `POSTGRES_PASSWORD` | Database password (`granit`) |
| `S3_ACCESS_KEY` / `S3_SECRET_KEY` | RustFS credentials (`rustfsadmin`) — change for any non-local deployment |
| `PUBLIC_ORIGIN` | Origin the desktop app reaches the service on (`http://localhost:8080`) |
| `CADDY_SITE_ADDRESS` | Caddy site address (`:8080`) |

The backend container itself is configured with `DATABASE_URL`,
`BIND_ADDR`, `S3_ENDPOINT`, `S3_PUBLIC_ENDPOINT`, `S3_BUCKET`,
`S3_ACCESS_KEY`, and `S3_SECRET_KEY` — wired up by the compose file.

### `S3_PUBLIC_ENDPOINT` — read this before deploying

Presigned upload URLs embed the host they were signed for. The server
talks to RustFS at the docker-internal `http://rustfs:9000`, but the URL
handed to the desktop app must point at the **public Caddy origin**, which
routes the `/granit-backups/*` bucket prefix to RustFS. If backups fail
with signature or connection errors on upload, `S3_PUBLIC_ENDPOINT`
(set from `PUBLIC_ORIGIN`) not matching the URL in Granit's settings is
the first thing to check.

## Deploying on a real domain

Set the domain and publish ports 80/443 instead of 8080:

```sh
# .env
PUBLIC_ORIGIN=https://backup.example.com
CADDY_SITE_ADDRESS=backup.example.com
S3_ACCESS_KEY=<random>
S3_SECRET_KEY=<random>
POSTGRES_PASSWORD=<random>
```

and change the caddy `ports:` mapping to `"80:80"` and `"443:443"`.
Caddy then provisions and renews the TLS certificate automatically.

### Behind Traefik or another reverse proxy

The public proxy can terminate HTTPS and forward HTTP to this stack's Caddy
listener. Set `PUBLIC_ORIGIN` and the desktop's backend URL to the public
HTTPS origin, and leave `CADDY_SITE_ADDRESS=:8080` for the internal listener.
The Rust server does not need native TLS for this arrangement. Keep the
proxy-to-backend connection on a trusted network and restrict access to the
internal listener. Use a publicly trusted certificate or install your private
CA on the desktop; disabling certificate verification defeats server identity
verification.

## Upload integrity

New uploads require the signed headers returned in `upload_headers`, including
the exact ciphertext size, SHA-256 checksum, and `If-None-Match: *`. The object
store must enforce S3 checksums and conditional PUTs and return SHA-256 checksums
on HEAD requests. Completion rejects missing or mismatched checksums. Each
archive is limited to 1 GiB; this is not an aggregate storage quota.

Upgrade desktop clients together with the server: older clients do not send
the required headers. Existing completed snapshots remain downloadable.
Delete and recreate pending uploads made by older clients if completion fails
because the object has no stored SHA-256 checksum. Previously issued upload
URLs retain their old permissions until their 15-minute expiry.

## Development

```sh
# Run a local postgres, then:
export DATABASE_URL=postgres://postgres:granit@localhost:5432/granit
cargo test -p granit-server

# After changing any sqlx query, refresh the committed offline data:
cd backend && cargo sqlx prepare -- --all-targets
```

Migrations are reversible sqlx-cli pairs in `migrations/` and run
automatically at server startup.

The live object-storage regression test is opt-in. Point it at a dedicated
test RustFS instance; it creates a temporary bucket and uploads dummy data:

```sh
GRANIT_TEST_S3_ENDPOINT=http://localhost:9000 \
GRANIT_TEST_S3_ACCESS_KEY=test-access-key \
GRANIT_TEST_S3_SECRET_KEY=test-secret-key \
cargo test -p granit-server object_store_enforces_upload_constraints -- --ignored
```

The test checks signed size and header enforcement, checksum rejection,
checksum retrieval, and write-once uploads. Verified with RustFS 1.0.0-rc.5.
