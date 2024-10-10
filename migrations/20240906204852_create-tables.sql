CREATE TABLE accounts_archive(
    id TEXT NOT NULL PRIMARY KEY,
    slot INT NOT NULL,
    data BLOB NOT NULL,
    executable BOOLEAN,
    lamports INTEGER,
    owner TEXT,
    rent_epoch INTEGER
);
CREATE UNIQUE INDEX idx_accounts_archive_id_slot ON accounts_archive (id,slot);

CREATE TABLE transactions(signature TEXT NOT NULL PRIMARY KEY, slot INT NOT NULL, err TEXT, memo TEXT, block_time INT, confirmation_status INT, data JSONB);
