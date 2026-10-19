-- Separate journal preserves provisioning_stages' terminal-result constraints.
CREATE TABLE provisioning_seed_regeneration (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES provisioning_intents(operation_id),
    journal_json TEXT NOT NULL CHECK(json_valid(journal_json) AND length(journal_json) BETWEEN 1 AND 4096)
);
