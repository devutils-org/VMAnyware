CREATE TABLE provisioning_cloud_init_dispatch (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES provisioning_intents(operation_id),
    config_digest TEXT NOT NULL CHECK(length(config_digest)=40 AND config_digest NOT GLOB '*[^0-9a-fA-F]*'),
    volume TEXT NOT NULL CHECK(length(volume) BETWEEN 3 AND 256),
    network TEXT NOT NULL CHECK(length(network) BETWEEN 1 AND 256)
);
