CREATE TABLE buckets (
    id UUID DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE files (
    id UUID DEFAULT gen_random_uuid(),
    extension TEXT,
    checksum TEXT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE bucket_files (
    name TEXT NOT NULL,
    bucket_id UUID NOT NULL,
    file_id UUID NOT NULL,
    PRIMARY KEY (bucket_id, file_id),
    FOREIGN KEY (bucket_id) REFERENCES buckets(id),
    FOREIGN KEY (file_id) REFERENCES files(id)
);


CREATE TABLE bucket_keys (
    id UUID DEFAULT gen_random_uuid(),
    key TEXT NOT NULL,
    bucket_id UUID NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (bucket_id) REFERENCES buckets(id)
);