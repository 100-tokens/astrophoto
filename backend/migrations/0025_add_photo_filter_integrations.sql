-- Per-filter integration breakdown (subs × exposure), independent of the
-- photo_filters tagging junction. Display-only acquisition detail; an
-- ordered JSON list of {filter, sub_count, sub_exposure_s}.
alter table photos
  add column filter_integrations jsonb not null default '[]'::jsonb;
