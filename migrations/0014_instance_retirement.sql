DROP TRIGGER IF EXISTS block_new_operations_on_deleted_instance;
-- Retain deleted associations while allowing a new identity to reuse their VMID.
CREATE TABLE instances_v14 (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    definition_name TEXT NOT NULL REFERENCES definitions(name),
    definition_hash TEXT NOT NULL CHECK (length(definition_hash) = 64),
    definition_revision INTEGER NOT NULL CHECK (definition_revision >= 1),
    definition_yaml TEXT NOT NULL,
    provider_scope TEXT NOT NULL CHECK (length(provider_scope) = 64),
    node TEXT NOT NULL,
    vmid INTEGER NOT NULL CHECK (vmid BETWEEN 100 AND 999999999),
    status TEXT NOT NULL DEFAULT 'registered' CHECK (status IN ('registered', 'deleted')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
INSERT INTO instances_v14 (id, definition_name, definition_hash, definition_revision, definition_yaml, provider_scope, node, vmid, status, created_at)
    SELECT id, definition_name, definition_hash, definition_revision, definition_yaml, provider_scope, node, vmid, status, created_at FROM instances;
DROP TABLE instances;
ALTER TABLE instances_v14 RENAME TO instances;
CREATE UNIQUE INDEX one_registered_provider_vmid ON instances(provider_scope, vmid)
    WHERE status='registered';

-- Old identities can be read for history but can never target a replacement VM.
CREATE TRIGGER block_new_operations_on_deleted_instance BEFORE INSERT ON operations
WHEN NEW.instance_id IS NOT NULL AND EXISTS (
    SELECT 1 FROM instances WHERE id=NEW.instance_id AND status='deleted'
) BEGIN SELECT RAISE(ABORT, 'instance_deleted'); END;
CREATE TRIGGER retain_deleted_instance BEFORE UPDATE ON instances
WHEN OLD.status='deleted'
BEGIN SELECT RAISE(ABORT, 'instance_deleted'); END;
