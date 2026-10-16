CREATE TABLE operations (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    request_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind = 'inventory_refresh'),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'interrupted')),
    result_json TEXT,
    failure_code TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((status = 'succeeded' AND result_json IS NOT NULL AND failure_code IS NULL)
        OR (status IN ('failed', 'interrupted') AND result_json IS NULL AND failure_code IS NOT NULL)
        OR (status IN ('queued', 'running') AND result_json IS NULL AND failure_code IS NULL))
);
CREATE UNIQUE INDEX one_active_inventory_refresh ON operations(kind)
    WHERE status IN ('queued', 'running');
