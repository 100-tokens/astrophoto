// Screen 4: /u/<handle>/p/<short> — photo fiche page with typed chips.

window.ScreenPhotoFiche = function ScreenPhotoFiche({ marks }) {
  const C = window.CATALOG;
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  const chips = [C.filterById.f1, C.filterById.f2, C.filterById.f3];
  const orphan = 'Astronomik CLS';

  // Use the real NGC 7000 SHO image
  const photo = window.PHOTOS.find(p => p.target.startsWith('NGC 7000')) || window.PHOTOS[7];

  const integ = [
    { f: C.filterById.f1, h: 5.5, frames: 66 },
    { f: C.filterById.f2, h: 4.7, frames: 56 },
    { f: C.filterById.f3, h: 4.3, frames: 52 },
  ];
  const totalH = integ.reduce((s, x) => s + x.h, 0);

  return (
    <div className="screen" data-screen-label="04 Photo · Fiche"
         style={{ width: 1440, height: 1380, overflow: 'hidden' }}>
      <AppHeader active="Gallery" auth marks={marks} />

      {/* Hero: full-bleed photo */}
      <section style={{ padding: '32px 64px 0', borderBottom: '1px solid var(--border-subtle)' }}>
        <Crumbs trail={[
          { label: 'Gallery' },
          { label: '@pascal' },
          { label: 'NGC 7000' },
        ]} />
        <div style={{ display: 'grid', gridTemplateColumns: '1fr auto', alignItems: 'end', marginTop: 24, marginBottom: 32, gap: 32 }}>
          <div>
            <div className="t-eyebrow">CYGNUS · EMISSION NEBULA · BORTLE 4</div>
            <h1 style={{ fontFamily: 'var(--font-display)', fontSize: 56, fontWeight: 400, margin: '12px 0 0', lineHeight: 1, letterSpacing: '-0.015em' }}>
              <em>NGC 7000</em> — North America in SHO
            </h1>
            <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginTop: 20, color: 'var(--fg-secondary)' }}>
              <div style={{ width: 28, height: 28, borderRadius: '50%', background: 'var(--accent)', color: 'var(--accent-ink)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: 'var(--font-display)', fontSize: 14, fontWeight: 600 }}>P</div>
              <span style={{ fontFamily: 'var(--font-display)', fontSize: 16 }}>Pascal Lechevallier</span>
              <span className="t-meta">·</span>
              <span className="t-meta">13 APRIL 2026 · SAINGHIN-EN-MÉLANTOIS</span>
            </div>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button className="btn btn-secondary btn-sm">♡ 184</button>
            <button className="btn btn-secondary btn-sm">Download</button>
            <button className="btn btn-primary btn-sm">Pin</button>
          </div>
        </div>
      </section>

      {/* Image */}
      <section style={{ padding: '32px 64px 0' }}>
        <div style={{ position: 'relative', aspectRatio: '3/2', maxWidth: '100%' }}>
          <Photo photo={photo} style={{ position: 'absolute', inset: 0 }} />
          <div className="corner-marks" style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}></div>
        </div>
      </section>

      {/* Bottom — chips + meta + gear */}
      <section style={{ padding: '48px 64px', display: 'grid', gridTemplateColumns: '1fr 380px', gap: 64 }}>
        <div>
          {/* Filter chips strip — the headline of this feature */}
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginBottom: 16 }}>
            <span className="t-label">FILTERS · MAPPED HUBBLE PALETTE</span>
            <span className="t-meta">3 TYPED · 1 LEGACY</span>
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, alignItems: 'center', marginBottom: 32 }}>
            {chips.map(f => <FilterChip key={f.id} filter={f} />)}
            <span className="fchip-orphan"><span className="lbl">legacy</span>{orphan}</span>
          </div>

          <div className="t-label" style={{ marginBottom: 12 }}>CAPTION</div>
          <div style={{ color: 'var(--fg-secondary)', fontSize: 15, lineHeight: 1.7, maxWidth: 640, fontFamily: 'var(--font-ui)' }}>
            <p style={{ marginTop: 0 }}>
              Three nights of clear skies in early April gave me a usable {totalH.toFixed(1)} hours of integration on NGC 7000. The SHO mapping pushes the dense Hα structure into the green channel and lets the OIII stand out in the Gulf of Mexico region.
            </p>
            <p>
              Star colors were rebuilt from a short RGB session with broadband filters — extracted with StarXTerminator and blended back linearly before stretching.
            </p>
          </div>

          {/* Acquisition table */}
          <div className="t-label" style={{ marginTop: 40, marginBottom: 12 }}>ACQUISITION · TECHNICAL</div>
          <table className="exif" style={{ maxWidth: 640 }}>
            <tbody>
              <tr><th>Total integration</th><td>{totalH.toFixed(1)} h across 3 nights</td></tr>
              <tr><th>Sub exposure</th><td>300 s · gain 100</td></tr>
              <tr><th>Frames</th><td>174 lights · 50 darks · 30 flats</td></tr>
              <tr><th>Right ascension</th><td>20ʰ 58ᵐ 47ˢ</td></tr>
              <tr><th>Declination</th><td>+44° 19′ 48″</td></tr>
              <tr><th>FOV · scale</th><td>2.51° × 1.68° · 1.41 ″/px</td></tr>
              <tr><th>Sky · moon</th><td>Bortle 4 · seeing 2.6″ · moon 14% · 38° away</td></tr>
            </tbody>
          </table>
        </div>

        {/* Right rail */}
        <aside style={{ display: 'flex', flexDirection: 'column', gap: 32 }}>
          {/* Per-filter integration */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 16 }}>
              <span className="t-label">INTEGRATION · PER FILTER</span>
              <span className="t-meta" style={{ color: 'var(--accent)' }}>{totalH.toFixed(1)} H</span>
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
              {integ.map(({ f, h, frames }) => {
                const pct = (h / totalH) * 100;
                const t = window.CATALOG.filterTypes[f.filter_type];
                return (
                  <div key={f.id}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
                      <FilterChip filter={f} compact />
                      <span style={{ flex: 1 }} />
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-primary)' }}>{h.toFixed(1)} h</span>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--fg-faint)' }}>· {frames}×300s</span>
                    </div>
                    <div style={{ height: 2, background: 'var(--border-subtle)' }}>
                      <div style={{ width: pct + '%', height: '100%', background: t.color }} />
                    </div>
                  </div>
                );
              })}
            </div>
            <div className="t-meta" style={{ marginTop: 14 }}>PER-FILTER INTEGRATION · ROADMAP PHASE 3</div>
          </div>

          {/* Gear */}
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 12 }}>
              <span className="t-label">GEAR</span>
              <a className="t-meta" style={{ color: 'var(--accent)' }}>VIEW SETUP →</a>
            </div>
            {[
              ['Telescope', 'Sky-Watcher Esprit 100 ED', '100 mm · 550 mm · f/5.5'],
              ['Camera',    'ZWO ASI2600MM Pro',         'Sony IMX571 · mono · 3.76 µm'],
              ['Mount',     'Sky-Watcher EQ6-R Pro',     'EQ German · 20 kg · GoTo'],
            ].map(([k, name, spec]) => (
              <a key={k} href="#" style={{
                display: 'block', textDecoration: 'none', color: 'inherit',
                padding: '12px 0', borderBottom: '1px dashed var(--border-subtle)',
              }}>
                <div className="t-label" style={{ color: 'var(--fg-faint)' }}>{k.toUpperCase()}</div>
                <div style={{ fontFamily: 'var(--font-display)', fontSize: 15, color: 'var(--fg-primary)', marginTop: 4 }}>{name}</div>
                <div className="t-meta" style={{ marginTop: 4 }}>{spec.toUpperCase()}</div>
              </a>
            ))}
            <div style={{ padding: '12px 0 0' }}>
              <span className="t-meta">SETUP</span>
              <a style={{ marginLeft: 8, fontFamily: 'var(--font-display)', fontStyle: 'italic', color: 'var(--fg-primary)' }}>Backyard SHO @ Bortle 4</a>
            </div>
          </div>
        </aside>
      </section>
    </div>
  );
};
