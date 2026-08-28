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

## Key management

```sh
docker compose exec backend granit-server list-keys
docker compose exec backend granit-server revoke-key --id <uuid>
```

Only SHA-256 hashes of keys are stored; a lost token cannot be recovered,
only replaced.

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
