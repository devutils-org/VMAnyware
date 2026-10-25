-- A creation that failed before any provider VM could exist releases its VMID.
-- Released associations stay readable for history but never target a VM again.
DROP TRIGGER IF EXISTS block_new_operations_on_deleted_instance;
DROP TRIGGER IF EXISTS validate_recreation_source;
DROP TRIGGER IF EXISTS block_new_operations_on_released_instance;
CREATE TABLE instances_v16 (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    definition_name TEXT NOT NULL REFERENCES definitions(name),
    definition_hash TEXT NOT NULL CHECK (length(definition_hash) = 64),
    definition_revision INTEGER NOT NULL CHECK (definition_revision >= 1),
    definition_yaml TEXT NOT NULL,
    provider_scope TEXT NOT NULL CHECK (length(provider_scope) = 64),
    node TEXT NOT NULL,
    vmid INTEGER NOT NULL CHECK (vmid BETWEEN 100 AND 999999999),
    status TEXT NOT NULL DEFAULT 'registered' CHECK (status IN ('registered', 'deleted', 'released')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
INSERT INTO instances_v16 SELECT * FROM instances;
DROP TABLE instances;
ALTER TABLE instances_v16 RENAME TO instances;
CREATE UNIQUE INDEX one_registered_provider_vmid ON instances(provider_scope, vmid)
    WHERE status='registered';

CREATE TRIGGER block_new_operations_on_deleted_instance BEFORE INSERT ON operations
WHEN NEW.instance_id IS NOT NULL AND EXISTS (
    SELECT 1 FROM instances WHERE id=NEW.instance_id AND status='deleted'
) BEGIN SELECT RAISE(ABORT, 'instance_deleted'); END;
CREATE TRIGGER block_new_operations_on_released_instance BEFORE INSERT ON operations
WHEN NEW.instance_id IS NOT NULL AND EXISTS (
    SELECT 1 FROM instances WHERE id=NEW.instance_id AND status='released'
) BEGIN SELECT RAISE(ABORT, 'instance_released'); END;
CREATE TRIGGER retain_deleted_instance BEFORE UPDATE ON instances
WHEN OLD.status IN ('deleted', 'released')
BEGIN SELECT RAISE(ABORT, 'instance_retired'); END;
CREATE TRIGGER validate_recreation_source BEFORE INSERT ON recreation_sources
WHEN NOT EXISTS (
    SELECT 1 FROM operations o JOIN instances target ON target.id=o.instance_id
    JOIN instances source ON source.id=NEW.source_instance_id
    WHERE o.id=NEW.operation_id AND o.kind='vm_provision'
      AND source.status='deleted' AND source.id<>target.id
      AND source.definition_name=target.definition_name
      AND source.definition_hash=target.definition_hash
      AND source.definition_revision=target.definition_revision
      AND source.definition_yaml=target.definition_yaml
) BEGIN SELECT RAISE(ABORT, 'invalid_recreation_source'); END;
