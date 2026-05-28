-- 0026 add PGC support: extend kind check, add position_angle_deg for
-- ellipse rendering, add a spatial index for cone search.

alter table targets
  drop constraint targets_kind_check;
alter table targets
  add constraint targets_kind_check
    check (kind in ('messier','ngc','ic','caldwell','common','other','pgc'));

alter table targets
  add column position_angle_deg real;

create index targets_radec_idx
    on targets (declination, right_ascension)
    where right_ascension is not null and declination is not null;
