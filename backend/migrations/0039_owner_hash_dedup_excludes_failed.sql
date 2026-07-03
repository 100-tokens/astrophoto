-- A finalize-stage failure leaves the row status='failed' — and the
-- per-owner hash dedup used to match it, so re-uploading the same file
-- 409'd "file already uploaded" forever (the upload page's Retry button
-- dead-ended the same way: DELETE /api/uploads/:id refused failed rows).
-- Failed rows no longer count as "already uploaded": the same bytes may
-- be re-initialized while the failed draft awaits discard or reclaim.
DROP INDEX photos_owner_hash_uidx;

CREATE UNIQUE INDEX photos_owner_hash_uidx
    ON photos (owner_id, original_hash)
    WHERE original_hash IS NOT NULL AND status <> 'failed';
