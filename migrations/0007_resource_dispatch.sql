ALTER TABLE provisioning_stages ADD COLUMN config_digest TEXT
    CHECK (config_digest IS NULL OR (
        kind = 'configure_resources' AND dispatch_started = 1
        AND length(config_digest) = 40 AND config_digest NOT GLOB '*[^0-9a-fA-F]*'
    ));
