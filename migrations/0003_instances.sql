CREATE TABLE instances (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    definition_name TEXT NOT NULL REFERENCES definitions(name),
    definition_hash TEXT NOT NULL CHECK (length(definition_hash) = 64),
    definition_revision INTEGER NOT NULL CHECK (definition_revision >= 1),
    definition_yaml TEXT NOT NULL,
    provider_scope TEXT NOT NULL CHECK (length(provider_scope) = 64),
    node TEXT NOT NULL,
    vmid INTEGER NOT NULL CHECK (vmid BETWEEN 100 AND 999999999),
    status TEXT NOT NULL DEFAULT 'registered' CHECK (status = 'registered'),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (provider_scope, vmid)
);
