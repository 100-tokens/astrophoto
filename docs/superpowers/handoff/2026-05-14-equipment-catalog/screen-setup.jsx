// Screen 2: /settings/equipment/new — setup builder with "Edit specs" panel.

const { useState: useStateS } = React;

function RoleRow({ kind, value, badge, children, expanded, onToggle }) {
  return (
    <div style={{
      borderTop: '1px solid var(--border-subtle)',
      padding: '20px 0',
    }}>
      <div style={{ display: 'grid', gridTemplateColumns: '140px 1fr auto', gap: 16, alignItems: 'center' }}>
        <span className="t-label">{kind}</span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <input className="input input-mono" defaultValue={value} />
          {badge && (
            <span className="chip" style={{ flexShrink: 0 }}>
              {badge}
            </span>
          )}
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button className="btn btn-ghost btn-sm" onClick={onToggle}>
            {expanded ? 'Hide specs' : 'Edit specs'}
          </button>
        </div>
      </div>
      {expanded && (
        <div style={{ marginLeft: 156, marginTop: 16 }}>{children}</div>
      )}
    </div>
  );
}

function SpecsPanel({ mode = 'edit', children, footerNote, onSave }) {
  const accentColor = mode === 'create' ? 'var(--accent)' : 'var(--warning)';
  const labelText = mode === 'create' ? 'NEW · WILL JOIN THE SHARED CATALOG' : 'EDITING A SHARED CATALOG ITEM';
  return (
    <div style={{
      border: '1px solid var(--border-default)',
      borderLeft: `2px solid ${accentColor}`,
      background: 'var(--bg-base)',
    }}>
      <div style={{
        padding: '10px 16px',
        borderBottom: '1px solid var(--border-subtle)',
        display: 'flex', alignItems: 'center', gap: 12,
      }}>
        <span className="t-label" style={{ color: accentColor }}>● {labelText}</span>
        <span style={{ flex: 1 }} />
        <span className="t-meta">SPECS HELP OTHERS FIND YOUR FRAMES</span>
      </div>
      <div style={{ padding: 20 }}>{children}</div>
      {(footerNote || onSave) && (
        <div style={{
          padding: '12px 16px',
          borderTop: '1px solid var(--border-subtle)',
          display: 'flex', alignItems: 'center', gap: 12,
        }}>
          {footerNote && <span className="t-meta" style={{ flex: 1 }}>{footerNote}</span>}
          <button className="btn btn-ghost btn-sm">Discard</button>
          <button className="btn btn-primary btn-sm">Save to catalog</button>
        </div>
      )}
    </div>
  );
}

window.ScreenSetupBuilder = function ScreenSetupBuilder({ marks }) {
  const C = window.CATALOG;
  const AppHeader = window.AppHeader;
  const [openTel, setOpenTel] = useStateS(true);
  const [openCam, setOpenCam] = useStateS(false);
  const [filters, setFilters] = useStateS([C.filterById.f1, C.filterById.f2, C.filterById.f3]);

  return (
    <div className="screen" data-screen-label="02 Setup · New"
         style={{ width: 1440, height: 1500, overflow: 'hidden' }}>
      <AppHeader active="Gallery" auth marks={marks} />

      <section style={{ padding: '40px 64px 24px', borderBottom: '1px solid var(--border-subtle)' }}>
        <Crumbs trail={[
          { label: 'Settings' },
          { label: 'Equipment' },
          { label: 'Setups' },
          { label: 'New setup' },
        ]} />
        <div className="t-eyebrow" style={{ marginTop: 16 }}>SETTINGS · EQUIPMENT</div>
        <h1 style={{ fontFamily: 'var(--font-display)', fontSize: 48, fontWeight: 400, margin: '8px 0 0', lineHeight: 1 }}>
          A new <em>setup</em>
        </h1>
        <p style={{ color: 'var(--fg-secondary)', maxWidth: 720, fontSize: 14, marginTop: 16 }}>
          A setup bundles the gear you use together for a session — telescope, camera, mount, filters. Apply it to a frame on upload and every field below populates in one click.
        </p>

        <div style={{ marginTop: 24 }}>
          <SubNav active="setups" items={[
            { key: 'profile',  label: 'Profile' },
            { key: 'gear',     label: 'My equipment', count: 14 },
            { key: 'setups',   label: 'Setups',       count: 3 },
            { key: 'sites',    label: 'Sites',        count: 2 },
            { key: 'plan',     label: 'Plan' },
          ]} />
        </div>
      </section>

      <section style={{ padding: '48px 64px', display: 'grid', gridTemplateColumns: '1fr 340px', gap: 48 }}>
        <div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 280px', gap: 24, marginBottom: 32 }}>
            <Field label="Setup name" full={false} mono={false}
                   hint="A short label — visible on every frame using this setup.">
              <input className="input" defaultValue="Backyard SHO @ Bortle 4" />
            </Field>
            <Field label="Default site" mono={false}
                   hint="Optional · prefills location on apply.">
              <select className="select" defaultValue="home">
                <option value="home">Backyard — Sainghin-en-Mélantois</option>
                <option>Remote — Spain (e-EyE)</option>
                <option>(none)</option>
              </select>
            </Field>
          </div>

          <div className="t-label" style={{ marginBottom: 8 }}>ROLES</div>

          <RoleRow kind="TELESCOPE" value="Sky-Watcher Esprit 100 ED"
                   badge="1,560 frames"
                   expanded={openTel} onToggle={() => setOpenTel(v => !v)}>
            <SpecsPanel mode="edit"
                        footerNote="Edits write to the shared catalog · 12 other users have this telescope.">
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 20 }}>
                <Field label="Design" mono={false}>
                  <select className="select" defaultValue="refractor_apo">
                    <option value="refractor_apo">Refractor APO</option>
                    <option>Refractor achro</option>
                    <option>SCT</option>
                    <option>RC</option>
                    <option>Newtonian</option>
                    <option>Maksutov-Cassegrain</option>
                    <option>Dall-Kirkham</option>
                  </select>
                </Field>
                <Field label="Aperture (mm)" mono value="100" />
                <Field label="Focal length (mm)" mono value="550" />
                <Field label="Focal ratio · computed" mono>
                  <input className="input input-mono" value="f/5.50" readOnly
                         style={{ background: 'var(--bg-raised)', color: 'var(--fg-muted)' }} />
                </Field>
                <div style={{ gridColumn: 'span 2' }}>
                  <Callout tone="info" label="DB-GENERATED">
                    <code style={{ fontFamily: 'var(--font-mono)' }}>focal_ratio_f</code> is a STORED column · <code style={{ fontFamily: 'var(--font-mono)' }}>focal_length_mm / aperture_mm</code> · not user-editable.
                  </Callout>
                </div>
              </div>
            </SpecsPanel>
          </RoleRow>

          <RoleRow kind="CAMERA" value="ZWO ASI2600MM Pro" badge="4,250 frames"
                   expanded={openCam} onToggle={() => setOpenCam(v => !v)} />

          <RoleRow kind="MOUNT" value="Sky-Watcher EQ6-R Pro" badge="3,120 frames"
                   expanded={false} onToggle={() => {}} />

          <RoleRow kind="FOCAL MODIFIER" value="" expanded={false} onToggle={() => {}} />

          {/* Filters role — multi-select chip input */}
          <div style={{ borderTop: '1px solid var(--border-subtle)', padding: '20px 0' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '140px 1fr', gap: 16, alignItems: 'flex-start' }}>
              <div style={{ paddingTop: 8 }}>
                <span className="t-label">FILTERS</span>
                <div className="t-meta" style={{ marginTop: 4 }}>MULTI · ORDERED</div>
              </div>
              <div>
                <FilterChipInput value={filters} onChange={setFilters} />
                <div className="t-meta" style={{ marginTop: 8 }}>
                  The filter list drives <code style={{ fontFamily: 'var(--font-mono)' }}>photo_filters</code> when this setup is applied. Order here is the canonical order shown on photos.
                </div>
              </div>
            </div>
          </div>

          {/* Save row */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginTop: 40, paddingTop: 24, borderTop: '1px solid var(--border-subtle)' }}>
            <span className="t-meta">5 ROLES · 3 FILTERS · READY TO APPLY</span>
            <span style={{ flex: 1 }} />
            <button className="btn btn-ghost btn-lg">Cancel</button>
            <button className="btn btn-primary btn-lg">Save setup</button>
          </div>
        </div>

        {/* Right rail */}
        <aside style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div style={{ padding: 20, border: '1px solid var(--border-subtle)', background: 'var(--bg-raised)' }}>
            <div className="t-label" style={{ marginBottom: 12 }}>SHARED CATALOG</div>
            <p style={{ margin: 0, fontSize: 13, color: 'var(--fg-secondary)', lineHeight: 1.6 }}>
              Equipment items are <strong style={{ color: 'var(--fg-primary)' }}>shared</strong> across astrophoto.pics. Editing specs on a role updates the catalog row for everyone using that item.
            </p>
            <p style={{ margin: '12px 0 0', fontSize: 12, color: 'var(--fg-muted)' }}>
              Phase 1: any signed-in user can edit. Moderation queue ships in phase 2.
            </p>
          </div>

          <div style={{ padding: 20, border: '1px solid var(--border-subtle)', background: 'var(--bg-raised)' }}>
            <div className="t-label" style={{ marginBottom: 16 }}>APPLY BEHAVIOR</div>
            <label style={{ display: 'flex', alignItems: 'flex-start', gap: 12, marginBottom: 16, cursor: 'pointer' }}>
              <input type="radio" name="apply" defaultChecked
                     style={{ accentColor: 'var(--accent)', marginTop: 3 }} />
              <span>
                <strong style={{ display: 'block', fontSize: 13 }}>Overwrite</strong>
                <span style={{ color: 'var(--fg-muted)', fontSize: 12 }}>Replace existing equipment fields and filter junction on apply.</span>
              </span>
            </label>
            <label style={{ display: 'flex', alignItems: 'flex-start', gap: 12, cursor: 'pointer' }}>
              <input type="radio" name="apply" style={{ accentColor: 'var(--accent)', marginTop: 3 }} />
              <span>
                <strong style={{ display: 'block', fontSize: 13 }}>Fill empty</strong>
                <span style={{ color: 'var(--fg-muted)', fontSize: 12 }}>Only set fields that are currently blank.</span>
              </span>
            </label>
          </div>
        </aside>
      </section>
    </div>
  );
};
