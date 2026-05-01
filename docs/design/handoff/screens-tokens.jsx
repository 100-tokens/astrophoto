/* ===== TOKENS ARTBOARD — design system showcase ===== */

window.ScreenTokens = function ScreenTokens({ marks }) {
  const Wordmark = marks.Wordmark;
  const Mark = marks.MarkReticle;

  const colors = [
    ["bg-canvas", "#0c0a08", "Page background"],
    ["bg-base", "#100d0a", "Default surface"],
    ["bg-raised", "#16120e", "Cards, panels"],
    ["bg-elevated", "#1d1812", "Hover, popovers"],
    ["border-subtle", "#221d17", "Hairlines"],
    ["border-default", "#2c2620", "Inputs, dividers"],
    ["border-strong", "#3a322a", "Buttons"],
    ["fg-primary", "#f4ece0", "Headlines, copy"],
    ["fg-secondary", "#c9bfae", "Body text"],
    ["fg-muted", "#8a8275", "Meta, captions"],
    ["fg-faint", "#5b554a", "Disabled"],
    ["accent", "#e8a43a", "Sodium amber"],
    ["accent-hover", "#f0b455", "Hover"],
    ["accent-press", "#c98920", "Active"],
  ];

  return (
    <div className="screen" style={{ width: "1440px", height: "1700px", overflow: "hidden", padding: "64px" }}>
      {/* Header */}
      <div style={{ paddingBottom: 40, borderBottom: "1px solid var(--border-default)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 }}>
          <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● ASTROPHOTO · DESIGN SYSTEM · v0.1</div>
          <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
            <Mark size={28} color="var(--accent)"/>
            <Wordmark size={24} italic={true}>Astrophoto</Wordmark>
          </div>
        </div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 88, fontWeight: 400, margin: 0, lineHeight: 0.95 }}>
          <em>Visual</em> language
        </h1>
        <p style={{ color: "var(--fg-secondary)", fontSize: 16, marginTop: 24, maxWidth: 640 }}>
          Observatory at 3am. Warm near-black, sodium amber, a refined transitional serif against
          a technical sans and mono. Every photo is the hero — the chrome recedes.
        </p>
      </div>

      {/* Type scale */}
      <div style={{ paddingTop: 48, paddingBottom: 32 }}>
        <div className="t-eyebrow" style={{ marginBottom: 24 }}>TYPOGRAPHY</div>
        <div style={{ display: "grid", gridTemplateColumns: "120px 1fr", gap: 24, alignItems: "baseline" }}>
          {[
            ["DISPLAY 88", { fontFamily: "var(--font-display)", fontSize: 88, fontStyle: "italic" }, "the work you make"],
            ["DISPLAY 56", { fontFamily: "var(--font-display)", fontSize: 56 }, "Cormorant Garamond — Regular & Italic"],
            ["TITLE 32", { fontFamily: "var(--font-display)", fontSize: 32 }, "NGC 7000 · North America Nebula"],
            ["BODY 16", { fontFamily: "var(--font-ui)", fontSize: 16, color: "var(--fg-secondary)" }, "IBM Plex Sans, the workhorse — for paragraphs and UI."],
            ["UI 14", { fontFamily: "var(--font-ui)", fontSize: 14 }, "Buttons, navigation, form labels."],
            ["MONO 13", { fontFamily: "var(--font-mono)", fontSize: 13 }, "180 × 360 s = 18.0 hours · gain 100 · −10 °C"],
            ["LABEL 11", { fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.16em", textTransform: "uppercase", color: "var(--fg-muted)" }, "MEASUREMENTS, NOT PROSE"],
          ].map(([label, st, sample]) => (
            <React.Fragment key={label}>
              <span className="t-meta" style={{ paddingTop: 12 }}>{label}</span>
              <span style={{...st, lineHeight: 1.2}}>{sample}</span>
            </React.Fragment>
          ))}
        </div>
      </div>

      {/* Color palette */}
      <div style={{ borderTop: "1px solid var(--border-subtle)", paddingTop: 48, paddingBottom: 32 }}>
        <div className="t-eyebrow" style={{ marginBottom: 24 }}>COLOR · DARK (DEFAULT)</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(7, 1fr)", gap: 12 }}>
          {colors.map(([name, hex, role]) => (
            <div key={name}>
              <div style={{ height: 88, background: hex, border: "1px solid var(--border-subtle)" }}/>
              <div style={{ marginTop: 8, fontFamily: "var(--font-mono)", fontSize: 11 }}>
                <div style={{ color: "var(--fg-primary)" }}>{name}</div>
                <div style={{ color: "var(--fg-muted)" }}>{hex}</div>
                <div style={{ color: "var(--fg-faint)", marginTop: 2 }}>{role}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Components */}
      <div style={{ borderTop: "1px solid var(--border-subtle)", paddingTop: 48, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 64 }}>
        <div>
          <div className="t-eyebrow" style={{ marginBottom: 24 }}>BUTTONS</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 16, alignItems: "flex-start" }}>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-primary btn-sm">Primary sm</button>
              <button className="btn btn-primary">Primary</button>
              <button className="btn btn-primary btn-lg">Primary lg</button>
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn btn-secondary btn-sm">Secondary</button>
              <button className="btn btn-secondary">Secondary</button>
              <button className="btn btn-ghost">Ghost</button>
              <button className="btn btn-danger">Delete</button>
            </div>
          </div>

          <div className="t-eyebrow" style={{ marginTop: 40, marginBottom: 24 }}>CHIPS / BADGES</div>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
            <span className="chip">Galaxies</span>
            <span className="chip chip-accent">● 248 ♡</span>
            <span className="chip">SHO narrowband</span>
            <span className="chip">Bortle 4</span>
            <span className="chip chip-accent">PUBLISHED</span>
          </div>
        </div>

        <div>
          <div className="t-eyebrow" style={{ marginBottom: 24 }}>FORM CONTROLS</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 16, maxWidth: 360 }}>
            <div><div className="t-label" style={{ marginBottom: 6 }}>EMAIL</div><input className="input" placeholder="you@somewhere.com"/></div>
            <div><div className="t-label" style={{ marginBottom: 6 }}>RA / DEC (MONO)</div><input className="input input-mono" defaultValue="20ʰ 58ᵐ 47ˢ / +44° 19′"/></div>
            <div><div className="t-label" style={{ marginBottom: 6 }}>CAPTION</div><textarea className="textarea" defaultValue="Narrowband, 18 h integration over 4 nights..."/></div>
          </div>
        </div>
      </div>
    </div>
  );
};
