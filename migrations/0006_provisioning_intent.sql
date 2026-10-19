CREATE TABLE operations_v6 (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    request_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('inventory_refresh', 'instance_observe', 'vm_start', 'vm_shutdown', 'vm_provision')),
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
        OR (kind IN ('instance_observe', 'vm_start', 'vm_shutdown', 'vm_provision') AND instance_id IS NOT NULL)),
    CHECK ((kind IN ('inventory_refresh', 'instance_observe') AND approval_json IS NULL
            AND input_hash IS NULL AND confirmed_at IS NULL AND dispatch_started = 0 AND provider_upid IS NULL)
        OR (kind IN ('vm_start', 'vm_shutdown', 'vm_provision') AND approval_json IS NOT NULL AND json_valid(approval_json)
            AND input_hash IS NOT NULL AND length(input_hash) = 64 AND confirmed_at IS NOT NULL AND confirmed_at >= 0)),
    CHECK (provider_upid IS NULL OR (dispatch_started = 1 AND length(provider_upid) BETWEEN 1 AND 512)),
    CHECK (status != 'queued' OR dispatch_started = 0),
    CHECK (status != 'uncertain' OR (kind IN ('vm_start', 'vm_shutdown', 'vm_provision') AND dispatch_started = 1)),
    CHECK ((status = 'succeeded' AND result_json IS NOT NULL AND failure_code IS NULL)
        OR (status IN ('failed', 'interrupted', 'uncertain') AND result_json IS NULL AND failure_code IS NOT NULL)
        OR (status IN ('queued', 'running') AND result_json IS NULL AND failure_code IS NULL))
);
INSERT INTO operations_v6 SELECT * FROM operations;
DROP TABLE operations;
ALTER TABLE operations_v6 RENAME TO operations;
CREATE UNIQUE INDEX one_active_inventory_refresh ON operations(kind)
    WHERE kind = 'inventory_refresh' AND status IN ('queued', 'running');
CREATE UNIQUE INDEX one_active_instance_observation ON operations(instance_id)
    WHERE kind = 'instance_observe' AND status IN ('queued', 'running');
CREATE UNIQUE INDEX one_active_instance_mutation ON operations(instance_id)
    WHERE kind IN ('vm_start', 'vm_shutdown', 'vm_provision') AND status IN ('queued', 'running', 'uncertain');

CREATE TABLE provisioning_intents (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES operations(id),
    catalog_yaml TEXT NOT NULL,
    plan_json TEXT NOT NULL CHECK(json_valid(plan_json))
);
CREATE TABLE provisioning_stages (
    operation_id TEXT NOT NULL REFERENCES provisioning_intents(operation_id),
    position INTEGER NOT NULL CHECK(position>=0),
    kind TEXT NOT NULL CHECK(kind IN ('full_clone','configure_resources','expand_disk','configure_cloud_init','start_vm','wait_for_guest_readiness','install_declared_packages','snapshot_after_provisioning')),
    status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN ('queued','running','succeeded','failed','interrupted','uncertain')),
    dispatch_started INTEGER NOT NULL DEFAULT 0 CHECK(dispatch_started IN (0,1)),
    provider_upid TEXT,
    result_json TEXT CHECK(result_json IS NULL OR json_valid(result_json)),
    PRIMARY KEY(operation_id,position),
    UNIQUE(operation_id,kind),
    CHECK(status!='queued' OR dispatch_started=0),
    CHECK(status!='uncertain' OR dispatch_started=1),
    CHECK(provider_upid IS NULL OR (dispatch_started=1 AND length(provider_upid) BETWEEN 1 AND 512)),
    CHECK((status='succeeded' AND result_json IS NOT NULL) OR (status!='succeeded' AND result_json IS NULL))
);
CREATE TRIGGER block_lifecycle_during_incomplete_provisioning BEFORE INSERT ON operations
WHEN NEW.kind IN ('vm_start','vm_shutdown') AND EXISTS (
    SELECT 1 FROM operations p WHERE p.kind='vm_provision' AND p.instance_id=NEW.instance_id AND p.status!='succeeded'
) BEGIN SELECT RAISE(ABORT,'provisioning_incomplete'); END;
