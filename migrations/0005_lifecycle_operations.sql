CREATE TABLE operations_v5 (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    request_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('inventory_refresh', 'instance_observe', 'vm_start', 'vm_shutdown')),
    instance_id TEXT REFERENCES instances(id),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'interrupted', 'uncertain')),
    result_json TEXT,
    failure_code TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    approval_json TEXT,
    input_hash TEXT,
    confirmed_at INTEGER,
    dispatch_started INTEGER NOT NULL DEFAULT 0 CHECK (dispatch_started IN (0, 1)),
    provider_upid TEXT,
    next_reconcile_at INTEGER NOT NULL DEFAULT 0,
    CHECK ((kind = 'inventory_refresh' AND instance_id IS NULL)
        OR (kind IN ('instance_observe', 'vm_start', 'vm_shutdown') AND instance_id IS NOT NULL)),
    CHECK ((kind IN ('inventory_refresh', 'instance_observe') AND approval_json IS NULL
            AND input_hash IS NULL AND confirmed_at IS NULL AND dispatch_started = 0 AND provider_upid IS NULL)
        OR (kind IN ('vm_start', 'vm_shutdown') AND approval_json IS NOT NULL AND json_valid(approval_json)
            AND input_hash IS NOT NULL AND length(input_hash) = 64 AND confirmed_at IS NOT NULL AND confirmed_at >= 0)),
    CHECK (provider_upid IS NULL OR (dispatch_started = 1 AND length(provider_upid) BETWEEN 1 AND 512)),
    CHECK (status != 'queued' OR dispatch_started = 0),
    CHECK (status != 'uncertain' OR (kind IN ('vm_start', 'vm_shutdown') AND dispatch_started = 1)),
    CHECK ((status = 'succeeded' AND result_json IS NOT NULL AND failure_code IS NULL)
        OR (status IN ('failed', 'interrupted', 'uncertain') AND result_json IS NULL AND failure_code IS NOT NULL)
        OR (status IN ('queued', 'running') AND result_json IS NULL AND failure_code IS NULL))
);
INSERT INTO operations_v5 (id, request_key, kind, instance_id, status, result_json, failure_code, created_at, updated_at)
    SELECT id, request_key, kind, instance_id, status, result_json, failure_code, created_at, updated_at FROM operations;
DROP TABLE operations;
ALTER TABLE operations_v5 RENAME TO operations;
CREATE UNIQUE INDEX one_active_inventory_refresh ON operations(kind)
    WHERE kind = 'inventory_refresh' AND status IN ('queued', 'running');
CREATE UNIQUE INDEX one_active_instance_observation ON operations(instance_id)
    WHERE kind = 'instance_observe' AND status IN ('queued', 'running');
CREATE UNIQUE INDEX one_active_instance_mutation ON operations(instance_id)
    WHERE kind IN ('vm_start', 'vm_shutdown') AND status IN ('queued', 'running', 'uncertain');
