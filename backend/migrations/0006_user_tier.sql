-- 0006 user tier: free / subscriber. Drives upload-size enforcement
-- in the presigned-PUT path. No billing UI in this phase; the column
-- is toggled manually for now.

alter table users
    add column tier text not null default 'free'
        check (tier in ('free', 'subscriber'));
