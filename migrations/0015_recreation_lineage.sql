-- A recreation is ordinary provisioning with an immutable retired source.
CREATE TABLE IF NOT EXISTS recreation_sources (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES operations(id),
    source_instance_id TEXT NOT NULL REFERENCES instances(id)
);
CREATE TRIGGER IF NOT EXISTS validate_recreation_source BEFORE INSERT ON recreation_sources
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
CREATE TRIGGER IF NOT EXISTS retain_recreation_source BEFORE UPDATE ON recreation_sources
BEGIN SELECT RAISE(ABORT, 'immutable_recreation_source'); END;
CREATE TRIGGER IF NOT EXISTS retain_recreation_source_history BEFORE DELETE ON recreation_sources
BEGIN SELECT RAISE(ABORT, 'immutable_recreation_source'); END;
