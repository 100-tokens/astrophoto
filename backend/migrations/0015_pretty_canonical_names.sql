-- 0015 prettify canonical_name for catalog rows whose name is the raw
-- zero-padded OpenNGC `Name` (e.g. "NGC0224", "IC0434"). The seed binary
-- now formats these on insert via pretty_name_fallback, but existing rows
-- in DBs already populated by 0014 + seed-targets need a one-shot update.
-- Idempotent: rows already in "NGC 224" form don't match the regex.
--
-- Manually-overridden canonical_names (e.g. "Andromeda Galaxy" for m31)
-- contain spaces or letters past the prefix and are excluded by the regex.

update targets
set canonical_name = upper(substring(canonical_name from '^([A-Za-z]+)'))
                     || ' '
                     || cast(cast(substring(canonical_name from '\d+$') as integer) as text)
where canonical_name ~ '^(NGC|IC|M)0*\d+$';
