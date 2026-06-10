-- Trigram indexes for target search (DB-9, June 2026 audit).
--
-- The autocomplete endpoint (/api/targets/autocomplete), site search, and
-- the /t target index all match `%q%` via ILIKE on slug / canonical_name /
-- aliases — predicates the btree indexes and the array-operator GIN index
-- (targets_aliases_gin_idx, 0010) cannot serve. With the PGC catalog seeded
-- (tens of thousands of rows) every keystroke was a sequential scan with
-- array unnesting.

create extension if not exists pg_trgm;

-- `array_to_string` is only STABLE, which generated columns (and expression
-- indexes) reject; wrap it in an IMMUTABLE function. Safe for text[]: no
-- per-element casts are involved, so the result depends only on the inputs.
create function immutable_array_to_string(text[], text)
    returns text
    language sql immutable parallel safe
    as $$ select array_to_string($1, $2) $$;

-- Denormalized space-joined alias list, maintained by Postgres itself.
-- Search predicates use `aliases_text ilike '%q%'` instead of unnesting,
-- which the trigram index below can serve.
alter table targets
    add column aliases_text text
    generated always as (immutable_array_to_string(aliases, ' ')) stored;

create index targets_slug_trgm_idx      on targets using gin (slug gin_trgm_ops);
create index targets_canonical_trgm_idx on targets using gin (canonical_name gin_trgm_ops);
create index targets_aliases_trgm_idx   on targets using gin (aliases_text gin_trgm_ops);
