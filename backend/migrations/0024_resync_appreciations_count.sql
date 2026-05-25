-- Re-sync the denormalized photos.appreciations_count to the true junction
-- count.
--
-- 0011 introduced the counter and backfilled it once, with a comment claiming
-- the appreciate/unappreciate handlers maintained it transactionally — but they
-- never did (they only wrote the `appreciations` junction). So every
-- appreciation between the 0011 backfill and the handler fix went uncounted,
-- and the feeds that sort/display by this column drifted from reality.
--
-- engagement/appreciations.rs now maintains the counter in the same
-- transaction as the junction write. This one-shot re-sync corrects the
-- accumulated drift; the handler maintenance keeps it correct thereafter.
update photos p
set appreciations_count = (
    select count(*) from appreciations a where a.photo_id = p.id
)
where appreciations_count <> (
    select count(*) from appreciations a where a.photo_id = p.id
);
