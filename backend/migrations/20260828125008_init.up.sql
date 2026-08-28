CREATE TABLE api_keys (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT NOT NULL,
    -- sha256 of the full bearer token; the token itself is never stored
    key_hash   BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at TIMESTAMPTZ
);

CREATE TABLE backups (
    id           UUID PRIMARY KEY,
    api_key_id   UUID NOT NULL REFERENCES api_keys (id),
    cave_name    TEXT NOT NULL,
    object_key   TEXT NOT NULL,
    size_bytes   BIGINT NOT NULL,
    sha256_hex   TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'pending',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX backups_key_created ON backups (api_key_id, created_at DESC);
