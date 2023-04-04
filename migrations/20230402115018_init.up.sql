CREATE TABLE buckets (
    id UUID DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE files (
    id UUID DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    extension TEXT,
    checksum TEXT NOT NULL,
    bucket_id UUID NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (bucket_id) REFERENCES buckets(id)
);

CREATE TABLE bucket_keys (
    id UUID DEFAULT gen_random_uuid(),
    key TEXT NOT NULL,
    bucket_id UUID NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (bucket_id) REFERENCES buckets(id)
);