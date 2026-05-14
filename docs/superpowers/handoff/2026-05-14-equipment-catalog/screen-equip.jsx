// Screen 3: /equip/filter/antlia-3nm-ha-pro — specs header above photo grid.

window.ScreenEquipBrowse = function ScreenEquipBrowse({ marks }) {
  const C = window.CATALOG;
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  const filter = C.filterById.f1; // Antlia 3nm Hα Pro
  const t = C.filterTypes[filter.filter_type];
  const sizeLabel = { '2in': '2 inch', '1_25in': '1.25 inch', '36mm': '36 mm', '50mm_round': '50 mm round' }[filter.size] || filter.size;

  // Map our 12 photosForAntliaHa to real photos from window.PHOTOS by index.
  const realPhotos = window.PHOTOS;
  const photos = C.photosForAntliaHa.map((p, i) => ({
    ...p,
    photo: realPhotos[(i * 3 + 1) % realPhotos.length],
  }));

  return (
    <div className="screen" data-screen-label="03 Equipment · Filter detail"
         style={{ width: 1440, height: 1620, overflow: 'hidden' }}>
      <AppHeader active="Gallery" auth marks={marks} />

      {/* Page header */}
      <section style={{ padding: '40px 64px 24px', borderBottom: '1px solid var(--border-subtle)' }}>
        <Crumbs trail={[
          { label: 'Equipment' },
          { label: 'Filters' },
          { label: 'Antlia 3nm Hα Pro' },
        ]} />
        <div style={{ display: 'grid', gridTemplateColumns: '1fr auto', gap: 48, alignItems: 'end', marginTop: 16 }}>
          <div>
            <div className="t-eyebrow">FILTER · NARROWBAND · 137 ITEMS IN CATALOG</div>
            <h1 style={{ fontFamily: 'var(--font-display)', fontSize: 64, fontWeight: 400, margin: '12px 0 0', lineHeight: 1, letterSpacing: '-0.015em' }}>
              Antlia 3 nm <em>Hα</em> Pro
            </h1>
            {/* Specs row — mono details */}
            <div style={{ display: 'flex', gap: 24, marginTop: 24, fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--fg-secondary)', flexWrap: 'wrap' }}>
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <FilterChip filter={filter} compact />
              </span>
              <span><span style={{ color: 'var(--fg-faint)' }}>BANDWIDTH</span> &nbsp; {filter.bandwidth_nm} nm</span>
              <span><span style={{ color: 'var(--fg-faint)' }}>SIZE</span> &nbsp; {sizeLabel}</span>
              <span><span style={{ color: 'var(--fg-faint)' }}>MOUNTED</span> &nbsp; {filter.mounted ? 'yes' : 'no'}</span>
              <span><span style={{ color: 'var(--fg-faint)' }}>BRAND</span> &nbsp; {filter.brand}</span>
            </div>
            <div className="t-meta" style={{ marginTop: 16 }}>
              {filter.usage_count.toLocaleString()} FRAMES · 218 SETUPS · 312 IMAGERS &nbsp; · &nbsp; APPROVED 2025-11-02 · SUBMITTED BY @KESTREL
            </div>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 12 }}>
            <div style={{ display: 'flex', gap: 8 }}>
              <button className="btn btn-ghost btn-sm">Follow</button>
              <button className="btn btn-secondary btn-sm">Edit specs</button>
              <button className="btn btn-primary btn-sm">Add to setup</button>
            </div>
            <span className="t-meta">item_id · 7c2 … d0e1</span>
          </div>
        </div>

        {/* Tabs */}
        <div style={{ display: 'flex', gap: 32, marginTop: 32 }}>
          {['Photos', 'Used with', 'Discussion', 'History'].map((tab, i) => (
            <a key={tab} className={'nav-link' + (i === 0 ? ' active' : '')}>
              {tab}
              {i === 0 && <span style={{ marginLeft: 8, color: 'var(--fg-faint)', fontSize: 10 }}>1,284</span>}
            </a>
          ))}
        </div>
      </section>

      {/* Photo grid + sidebar */}
      <section style={{ padding: '48px 64px', display: 'grid', gridTemplateColumns: '1fr 320px', gap: 48 }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 24 }}>
            <span className="t-label">FRAMES USING THIS FILTER</span>
            <span style={{ flex: 1 }} />
            <button className="chip">SORT · MOST RECENT ▾</button>
            <button className="chip">PALETTE · ANY ▾</button>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 16 }}>
            {photos.map((p, i) => (
              <a key={p.id} href="#" style={{ display: 'flex', flexDirection: 'column', gap: 10, textDecoration: 'none', color: 'inherit' }}>
                <div style={{ aspectRatio: '4/3', position: 'relative', overflow: 'hidden' }}>
                  <Photo photo={p.photo} style={{ position: 'absolute', inset: 0 }} />
                </div>
                <div>
                  <div style={{ fontFamily: 'var(--font-display)', fontSize: 14, fontStyle: 'italic', color: 'var(--fg-primary)' }}>{p.title}</div>
                  <div className="t-meta" style={{ marginTop: 4, display: 'flex', justifyContent: 'space-between' }}>
                    <span>@{p.handle.toUpperCase()}</span>
                    <span>{p.integ_h.toFixed(1)} H · B{p.bortle}</span>
                  </div>
                </div>
              </a>
            ))}
          </div>
        </div>

        {/* Sidebar */}
        <aside style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
          <div>
            <div className="t-label" style={{ marginBottom: 12 }}>OTHER ANTLIA NARROWBAND</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {[C.filterById.f2, C.filterById.f3].map(f => (
                <a key={f.id} href="#" style={{
                  display: 'flex', alignItems: 'center', gap: 12,
                  padding: '10px 12px', border: '1px solid var(--border-subtle)', background: 'var(--bg-raised)',
                  textDecoration: 'none', color: 'inherit',
                }}>
                  <FilterChip filter={f} compact />
                  <span style={{ flex: 1 }} />
                  <span className="t-meta">{f.usage_count.toLocaleString()}</span>
                </a>
              ))}
            </div>
          </div>

          <div>
            <div className="t-label" style={{ marginBottom: 12 }}>TRANSMISSION CURVE</div>
            <div style={{ padding: 16, border: '1px solid var(--border-subtle)', background: 'var(--bg-raised)' }}>
              <svg viewBox="0 0 240 110" width="100%" height="110" style={{ display: 'block' }}>
                <defs>
                  <linearGradient id="haGrad" x1="0" x2="0" y1="0" y2="1">
                    <stop offset="0%"   stopColor={t.color} stopOpacity="0.5"/>
                    <stop offset="100%" stopColor={t.color} stopOpacity="0"/>
                  </linearGradient>
                </defs>
                <line x1="20" y1="95" x2="235" y2="95" stroke="var(--border-default)" strokeWidth="1"/>
                <line x1="20" y1="10" x2="20"  y2="95" stroke="var(--border-default)" strokeWidth="1"/>
                <path d="M20 95 L150 95 Q158 95 160 18 Q162 95 170 95 L235 95 Z" fill="url(#haGrad)"/>
                <path d="M20 95 L150 95 Q158 95 160 18 Q162 95 170 95 L235 95" fill="none" stroke={t.color} strokeWidth="1.6"/>
                <text x="20"  y="108" fontFamily="JetBrains Mono" fontSize="9" fill="var(--fg-faint)">400</text>
                <text x="125" y="108" fontFamily="JetBrains Mono" fontSize="9" fill="var(--fg-faint)">656</text>
                <text x="208" y="108" fontFamily="JetBrains Mono" fontSize="9" fill="var(--fg-faint)">700 nm</text>
              </svg>
              <div className="t-meta" style={{ marginTop: 8 }}>COMMUNITY-SUBMITTED · MARCH 2026</div>
            </div>
          </div>

          <div>
            <div className="t-label" style={{ marginBottom: 12 }}>ITEM META</div>
            <table className="exif">
              <tbody>
                <tr><th>kind</th><td>filter</td></tr>
                <tr><th>status</th><td style={{ color: 'var(--success)' }}>approved</td></tr>
                <tr><th>canonical</th><td>antlia 3nm ha pro</td></tr>
                <tr><th>created</th><td>2025-10-18</td></tr>
                <tr><th>approved</th><td>2025-11-02</td></tr>
                <tr><th>submitted_by</th><td>@kestrel</td></tr>
              </tbody>
            </table>
          </div>
        </aside>
      </section>
    </div>
  );
};
