-- 0019 setup default_apply_mode: per-setup default for apply-setup mode.
-- Backs the "APPLY BEHAVIOR" radios on the setup builder UI. The
-- apply-setup endpoint itself still requires a `mode` in the request
-- body; this column just lets the frontend remember the user's
-- preferred default for each setup.

alter table equipment_setups
    add column default_apply_mode text not null default 'overwrite'
        check (default_apply_mode in ('overwrite', 'fill_empty'));
