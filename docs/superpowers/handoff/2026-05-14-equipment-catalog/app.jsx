// Main wiring — DesignCanvas with the four catalog screens + Tweaks.

const { useEffect: useEffectA } = React;

const APP_TWEAKS = /*EDITMODE-BEGIN*/{
  "chipStyle": "vivid",
  "showFilterPalette": true
}/*EDITMODE-END*/;

// Reference artboard — every filter type rendered as a chip in the
// current style, so the system vocabulary is comparable side-by-side.
function FilterTypePalette() {
  const types = window.CATALOG.filterTypes;
  const samples = Object.entries(types).map(([key, t]) => {
    const f = { id: 'pal-' + key, display_name: t.label, filter_type: key };
    if (['h_alpha', 'oiii', 'sii'].includes(key)) f.bandwidth_nm = 3;
    if (['dual_band', 'tri_band', 'quad_band'].includes(key)) f.bandwidth_nm = 5;
    return f;
  });
  const C = window.CATALOG;
  return (
    <div className="screen" style={{ width: 1200, height: 880, padding: 48 }}>
      <div className="t-eyebrow">FILTER · TYPE PALETTE</div>
      <h2 style={{ fontFamily: 'var(--font-display)', fontSize: 36, fontWeight: 400, margin: '12px 0 0', lineHeight: 1.05 }}>
        14 <em>typed</em> states + untyped
      </h2>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 48, marginTop: 40 }}>
        <div>
          <div className="t-label" style={{ marginBottom: 12 }}>ALL TYPES</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            {samples.map(f => <FilterChip key={f.id} filter={f} />)}
            <FilterChip filter={{ id: 'pal-u', display_name: 'Astronomik CLS', filter_type: null }} />
          </div>

          <div className="t-label" style={{ marginTop: 32, marginBottom: 12 }}>IN CONTEXT</div>
          <div style={{ marginBottom: 16 }}>
            <span className="t-meta">SHO NARROWBAND</span>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 6 }}>
              {[C.filterById.f1, C.filterById.f2, C.filterById.f3].map(f => <FilterChip key={f.id} filter={f} />)}
            </div>
          </div>
          <div style={{ marginBottom: 16 }}>
            <span className="t-meta">LRGB · MONO BROADBAND</span>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 6 }}>
              {[C.filterById.f10, C.filterById.f7, C.filterById.f8, C.filterById.f9].map(f => <FilterChip key={f.id} filter={f} />)}
            </div>
          </div>
          <div>
            <span className="t-meta">OSC DUAL-BAND</span>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 6 }}>
              {[C.filterById.f11].map(f => <FilterChip key={f.id} filter={f} />)}
            </div>
          </div>
        </div>

        <div style={{ borderLeft: '1px solid var(--border-subtle)', paddingLeft: 48 }}>
          <div className="t-label" style={{ marginBottom: 12 }}>STYLE MODES · TOGGLE VIA TWEAKS</div>
          {[
            ['vivid',   'Tinted background + colored badge. High recognition.'],
            ['outline', 'Ring-only chip; quieter, lets type breathe.'],
            ['mono',    'Neutral chip — the type code is the only signal.'],
          ].map(([mode, desc]) => (
            <div key={mode} style={{ marginBottom: 20 }}>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 6 }}>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-primary)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.08em' }}>{mode}</span>
                <span className="t-meta">{desc.toUpperCase()}</span>
              </div>
              <div data-ap-chip={mode} style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                {[C.filterById.f1, C.filterById.f2, C.filterById.f3, C.filterById.f10, C.filterById.f11].map(f =>
                  <FilterChip key={f.id} filter={f} />)}
              </div>
            </div>
          ))}

          <div className="t-label" style={{ marginTop: 32, marginBottom: 12 }}>TYPE → SODIUM-WARM COLOR</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 4, fontFamily: 'var(--font-mono)', fontSize: 11 }}>
            {Object.entries(window.CATALOG.filterTypes).map(([k, t]) => (
              <div key={k} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ width: 10, height: 10, background: t.color, border: '1px solid var(--border-default)' }}></span>
                <span style={{ color: 'var(--fg-muted)', minWidth: 50 }}>{t.code.toUpperCase()}</span>
                <span style={{ color: 'var(--fg-secondary)' }}>{t.label}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function App() {
  const [t, setTweak] = useTweaks(APP_TWEAKS);
  const marks = window.AstroMarks;

  return (
    <>
      <DesignCanvas>
        <DCSection
          id="hero"
          title="Equipment catalog · upload flow"
          subtitle="The hero surface: a structured filter chip input replaces the legacy free-text Filters field on /upload/verify."
        >
          <DCArtboard id="verify" label="A · Upload · Verify (filter chip input)" width={1440} height={1320}>
            <div data-ap-chip={t.chipStyle} style={{ width: '100%', height: '100%' }}>
              <ScreenUploadVerify marks={marks} />
            </div>
          </DCArtboard>
        </DCSection>

        <DCSection
          id="catalog"
          title="Catalog read & write"
          subtitle="Browse the typed catalog item · build a setup with editable specs that write back to the shared catalog."
        >
          <DCArtboard id="browse" label="B · /equip/filter/<slug> (specs header)" width={1440} height={1620}>
            <div data-ap-chip={t.chipStyle} style={{ width: '100%', height: '100%' }}>
              <ScreenEquipBrowse marks={marks} />
            </div>
          </DCArtboard>
          <DCArtboard id="setup" label="C · /settings/equipment/new (Edit specs panel)" width={1440} height={1500}>
            <div data-ap-chip={t.chipStyle} style={{ width: '100%', height: '100%' }}>
              <ScreenSetupBuilder marks={marks} />
            </div>
          </DCArtboard>
        </DCSection>

        <DCSection
          id="consume"
          title="Where it lands"
          subtitle="The fiche photo renders typed chips from the junction · legacy orphan tokens trail behind."
        >
          <DCArtboard id="photo" label="D · /u/<handle>/p/<short> (typed chips + integration)" width={1440} height={1380}>
            <div data-ap-chip={t.chipStyle} style={{ width: '100%', height: '100%' }}>
              <ScreenPhotoFiche marks={marks} />
            </div>
          </DCArtboard>
        </DCSection>

        <DCSection
          id="system"
          title="Design vocabulary"
          subtitle="14 typed states + the three style modes (vivid · outline · mono) controlled via Tweaks."
        >
          <DCArtboard id="palette" label="E · Filter type palette" width={1200} height={880}>
            <FilterTypePalette />
          </DCArtboard>
        </DCSection>
      </DesignCanvas>

      <TweaksPanel title="Tweaks">
        <TweakSection label="Filter chip style">
          <TweakRadio
            label="Style"
            value={t.chipStyle}
            options={[
              { value: 'vivid',   label: 'Vivid' },
              { value: 'outline', label: 'Outline' },
              { value: 'mono',    label: 'Mono' },
            ]}
            onChange={(v) => setTweak('chipStyle', v)}
          />
        </TweakSection>
      </TweaksPanel>
    </>
  );
}

ReactDOM.createRoot(document.getElementById('root')).render(<App />);
