-- Super-admin role.
--
-- A single boolean privilege level: `is_admin = true` grants access to the
-- `/api/admin/*` surface (manage the equipment catalog and app settings).
-- It is orthogonal to `tier` (a free-tier user can be an admin). Admin status
-- is read from this column during request auth (the `AdminUser` extractor),
-- not stored in the session, so revoking it takes effect on the next request.
alter table users add column is_admin boolean not null default false;

-- Bootstrap the first/owner admin. This is the app owner's account; the
-- statement is idempotent and no-ops on databases where the account does not
-- exist (e.g. dev/staging). New admins are granted from the admin UI later.
update users set is_admin = true where email = 'pascal@leclech.fr';
