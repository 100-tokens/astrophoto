// Screen 1: /upload/[id]/verify — structured filter chip input lives here.
// Rebuilt on the Astrophoto design system vocabulary.

const { useState: useStateV } = React;

window.ScreenUploadVerify = function ScreenUploadVerify({ marks }) {
  const C = window.CATALOG;
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;

  const [filters, setFilters] = useStateV([
    C.filterById.f1, // Antlia Hα
    C.filterById.f2, // Antlia OIII
    C.filterById.f3, // Antlia SII
  ]);
  const orphans = ['Astronomik CLS'];
  const [setupApplied, setSetupApplied] = useStateV(true);

  // Pick a sample photo from the curated set — NGC 7000 makes sense for SHO.
  const photo = window.PHOTOS.find(p => p.target.startsWith('NGC 7000')) || window.PHOTOS[7];

  return (
    <div className="screen" data-screen-label="01 Upload · Verify"
         style={{ width: 1440, height: 1320, overflow: 'hidden' }}>
      <AppHeader active="Gallery" auth marks={marks} />

      {/* Title + stepper */}
      <section style={{ padding: '40px 64px 24px', borderBottom: '1px solid var(--border-subtle)' }}>
        <Crumbs trail={[
          { label: 'Upload' },
          { label: 'Session · 12 Apr · 7 frames' },
          { label: 'Verify' },
        ]} />
        <div className="t-eyebrow" style={{ marginTop: 16 }}>NEW FRAME · 02 OF 07</div>
        <h1 style={{ fontFamily: 'var(--font-display)', fontSize: 48, fontWeight: 400, margin: '8px 0 0', lineHeight: 1 }}>
          Verify <em>NGC 7000</em> — North America
        </h1>

        <div style={{ display: 'flex', gap: 0, marginTop: 32, fontFamily: 'var(--font-mono)', fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase' }}>
          {[['01', 'UPLOAD', 'done'], ['02', 'VERIFY DATA', 'active'], ['03', 'EQUIPMENT', 'active'], ['04', 'CAPTION & PUBLISH', '']].map(([n, l, s]) => (
            <div key={n} style={{
              flex: 1, padding: '16px 0',
              borderTop: `2px solid ${s ? 'var(--accent)' : 'var(--border-default)'}`,
              color: s ? 'var(--fg-primary)' : 'var(--fg-muted)',
              display: 'flex', gap: 12, alignItems: 'center',
            }}>
              <span style={{ color: s ? 'var(--accent)' : 'var(--fg-faint)' }}>{n}</span>
              <span>{l}</span>
              {s === 'done' && <span style={{ color: 'var(--accent)', marginLeft: 'auto', marginRight: 32 }}>✓</span>}
            </div>
          ))}
        </div>
      </section>

      <section style={{ padding: '48px 64px', display: 'grid', gridTemplateColumns: '520px 1fr', gap: 64 }}>
        {/* Left — image preview + EXIF */}
        <div>
          <div className="t-label" style={{ marginBottom: 12 }}>FRAME</div>
          <div style={{ position: 'relative', aspectRatio: '4/3' }}>
            <Photo photo={photo} style={{ position: 'absolute', inset: 0 }} />
          </div>
          <div className="t-meta" style={{ marginTop: 12, display: 'flex', justifyContent: 'space-between' }}>
            <span>NGC7000_SHO_v3.tif</span>
            <span>72.1 MB · 6248 × 4176</span>
          </div>

          <div style={{ marginTop: 32 }}>
            <div className="t-label" style={{ marginBottom: 12 }}>DETECTED FROM EXIF</div>
            <table className="exif">
              <tbody>
                <tr><th>Camera</th><td>ZWO ASI2600MM Pro</td></tr>
                <tr><th>Sensor</th><td>Sony IMX571 · mono</td></tr>
                <tr><th>Sub exposure</th><td>300 s</td></tr>
                <tr><th>Gain · Offset</th><td>100 · 50</td></tr>
                <tr><th>Sensor temp</th><td>−10 °C</td></tr>
                <tr><th>Frames captured</th><td>174 lights</td></tr>
              </tbody>
            </table>
          </div>
        </div>

        {/* Right — form */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 8 }}>
            <div className="t-label">CONFIRM &amp; PUBLISH</div>
            <span className="t-meta" style={{ color: 'var(--accent)' }}>● 11 fields recovered from EXIF</span>
          </div>
          <p style={{ color: 'var(--fg-secondary)', fontSize: 13, marginTop: 0, marginBottom: 32 }}>
            Pick filters from the catalog — types travel with them. Correct anything that's wrong.
          </p>

          {/* Setup row */}
          <div style={{ marginBottom: 28 }}>
            <div className="t-label" style={{ marginBottom: 8 }}>SETUP</div>
            {setupApplied ? (
              <div style={{
                display: 'flex', alignItems: 'center', gap: 12,
                padding: '10px 14px',
                background: 'var(--bg-accent-tint)',
                border: '1px solid var(--border-default)',
                borderLeft: '2px solid var(--accent)',
              }}>
                <span style={{
                  width: 8, height: 8, background: 'var(--accent)',
                }}></span>
                <span style={{ fontFamily: 'var(--font-display)', fontStyle: 'italic', fontSize: 15, whiteSpace: 'nowrap' }}>Backyard SHO @ Bortle 4</span>
                <span className="t-meta">ESPRIT 100 · 2600MM · ANTLIA 3NM × 3</span>
                <span style={{ flex: 1 }} />
                <span className="t-meta" style={{ color: 'var(--fg-faint)' }}>setup_id · 0e1f…a7</span>
                <button className="btn btn-ghost btn-sm" onClick={() => setSetupApplied(false)}>Detach</button>
              </div>
            ) : (
              <input className="input" placeholder="Pick a setup or fill manually…" />
            )}
          </div>

          {/* Equipment grid */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 20 }}>
            <Field label="Telescope" value="Sky-Watcher Esprit 100 ED" mono detected />
            <Field label="Camera"    value="ZWO ASI2600MM Pro" mono detected />
            <Field label="Mount"     value="Sky-Watcher EQ6-R Pro" mono detected="auto" />
            <Field label="Focal modifier" mono detected="auto">
              <input className="input input-mono" placeholder="(none)" />
            </Field>

            {/* HERO — filter chip input */}
            <Field label="FILTERS · ORDERED · STRUCTURED" full
                   hint={<span>Writes the <span style={{ color: 'var(--fg-secondary)' }}>photo_filters</span> junction. Cache string rebuilt in same tx. Drag to reorder · position 0 appears first on the photo page.</span>}>
              <FilterChipInput value={filters} onChange={setFilters} orphans={orphans} startOpen={true} />
            </Field>
          </div>

          {/* Acquisition */}
          <div className="t-label" style={{ marginTop: 36, marginBottom: 12 }}>ACQUISITION</div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 16 }}>
            <Field label="Light frames"      value="174"     mono detected />
            <Field label="Sub exposure"      value="300 s"   mono detected />
            <Field label="Total integration" value="14.5 h"  mono />
            <Field label="Stacking" mono>
              <select className="select input-mono" defaultValue="PixInsight">
                <option>PixInsight</option><option>Siril</option><option>APP</option>
              </select>
            </Field>
          </div>

          <div style={{ marginTop: 24 }}>
            <Field label="Notes (optional)" full mono={false}>
              <textarea className="textarea" defaultValue="SHO mapped to Hubble palette. Stars from a short RGB session under broadband filters, blended back with StarXTerminator." />
            </Field>
          </div>

          {/* Footer */}
          <div style={{ marginTop: 40, display: 'flex', gap: 12, alignItems: 'center' }}>
            <span className="t-meta">3 FILTERS · 1 ORPHAN · CACHE WILL REBUILD</span>
            <span style={{ flex: 1 }} />
            <button className="btn btn-ghost btn-lg">Save as draft</button>
            <button className="btn btn-secondary btn-lg">← Previous</button>
            <button className="btn btn-primary btn-lg">Save &amp; next →</button>
          </div>
        </div>
      </section>
    </div>
  );
};
