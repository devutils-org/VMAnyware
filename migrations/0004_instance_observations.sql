CREATE TABLE operations_v4 (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    request_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('inventory_refresh', 'instance_observe')),
    instance_id TEXT REFERENCES instances(id),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'interrupted')),
    result_json TEXT,
    failure_code TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((kind = 'inventory_refresh' AND instance_id IS NULL)
        OR (kind = 'instance_observe' AND instance_id IS NOT NULL)),
    CHECK ((status = 'succeeded' AND result_json IS NOT NULL AND failure_code IS NULL)
        OR (status IN ('failed', 'interrupted') AND result_json IS NULL AND failure_code IS NOT NULL)
        OR (status IN ('queued', 'running') AND result_json IS NULL AND failure_code IS NULL))
);
INSERT INTO operations_v4 (id, request_key, kind, status, result_json, failure_code, created_at, updated_at)
    SELECT id, request_key, kind, status, result_json, failure_code, created_at, updated_at FROM operations;
DROP TABLE operations;
ALTER TABLE operations_v4 RENAME TO operations;
CREATE UNIQUE INDEX one_active_inventory_refresh ON operations(kind)
    WHERE kind = 'inventory_refresh' AND status IN ('queued', 'running');
CREATE UNIQUE INDEX one_active_instance_observation ON operations(instance_id)
    WHERE kind = 'instance_observe' AND status IN ('queued', 'running');
