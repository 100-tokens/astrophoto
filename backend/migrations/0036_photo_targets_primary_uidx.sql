-- One primary target per photo (DB-10, June 2026 audit).
--
-- The legacy freetext attach path (targets::attach_primary_by_freetext)
-- upserted `is_primary = true` rows without demoting the previous primary,
-- so photos edited through it could accumulate several primaries, making
-- "the" primary target order-dependent. Demote all but the newest row
-- (created_at, target_id as the deterministic tie-break), then enforce the
-- invariant with a partial unique index. Writers must demote-then-promote
-- inside one transaction from now on.

update photo_targets pt
   set is_primary = false
 where pt.is_primary
   and exists (
        select 1 from photo_targets p2
         where p2.photo_id = pt.photo_id
           and p2.is_primary
           and (p2.created_at, p2.target_id) > (pt.created_at, pt.target_id)
   );

create unique index photo_targets_primary_uidx
    on photo_targets (photo_id)
 where is_primary;
