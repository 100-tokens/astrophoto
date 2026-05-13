# OpenNGC catalog snapshot

**Source:** https://github.com/mattiaverga/OpenNGC
**Pinned commit:** 36cb178a0f69dba8bfc03a99c10512831edf1c6b
**Pinned date:** 2026-05-06
**License:** CC-BY-SA 4.0 — https://creativecommons.org/licenses/by-sa/4.0/

## Files

- `NGC.csv` — main catalog (~14k galaxies, nebulae, clusters)
- `addendum.csv` — Messier objects not in NGC/IC core catalog

## Refresh procedure

```bash
COMMIT=$(curl -sSL https://api.github.com/repos/mattiaverga/OpenNGC/commits/master | jq -r '.sha')
curl -sSL "https://raw.githubusercontent.com/mattiaverga/OpenNGC/${COMMIT}/database_files/NGC.csv" \
  -o backend/data/openngc/NGC.csv
curl -sSL "https://raw.githubusercontent.com/mattiaverga/OpenNGC/${COMMIT}/database_files/addendum.csv" \
  -o backend/data/openngc/addendum.csv
# Update the pinned commit/date in this README, then commit.
just seed-targets  # apply to dev DB
```

## Attribution (CC-BY-SA 4.0)

User-facing pages displaying this data must show:
- Attribution: "OpenNGC by Mattia Verga and contributors"
- License link: https://creativecommons.org/licenses/by-sa/4.0/
- Source link: https://github.com/mattiaverga/OpenNGC
- Change indication: "Adapted to slug format and merged with manual catalog seed."
