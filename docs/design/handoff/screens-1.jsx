/* ===== ASTROPHOTO SCREENS =====
   Each screen is a self-contained component sized for its artboard.
*/

const { useState } = React;
const PHOTOS = window.PHOTOS;

/* ============================================================
   1. PUBLIC GALLERY / LANDING — desktop, 1440 wide
   ============================================================ */
window.ScreenGallery = function ScreenGallery({ marks }) {
  const AppHeader = window.AppHeader;
  const AppFooter = window.AppFooter;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1900px", overflow: "hidden" }}>
      <AppHeader active="Gallery" marks={marks}/>

      {/* Hero strip — editorial */}
      <section style={{
        padding: "72px 64px 48px",
        display: "grid",
        gridTemplateColumns: "1fr 1fr",
        gap: "64px",
        alignItems: "end",
        borderBottom: "1px solid var(--border-subtle)",
      }}>
        <div>
          <div className="t-eyebrow" style={{ marginBottom: "16px" }}>
            <span style={{ color: "var(--accent)" }}>●</span> 14 March 2026 · Friday
          </div>
          <h1 style={{
            fontFamily: "var(--font-display)",
            fontSize: "64px",
            lineHeight: 1.05,
            margin: 0,
            fontWeight: 600,
            letterSpacing: "-0.015em",
          }}>
            A quiet archive<br/>
            of <em>the night sky</em>,<br/>
            kept by those who watch it.
          </h1>
          <p style={{
            marginTop: "32px",
            fontSize: "16px",
            lineHeight: 1.6,
            color: "var(--fg-secondary)",
            maxWidth: "520px",
          }}>
            Astrophoto is a home for amateur astrophotographers — a place where
            an 18-hour integration of NGC 7000 looks as monumental as it actually is,
            and where every frame carries its full record: target, equipment, sky.
          </p>
          <div style={{ marginTop: "32px", display: "flex", gap: "16px" }}>
            <button className="btn btn-primary btn-lg">Open an account</button>
            <button className="btn btn-secondary btn-lg">Browse the gallery →</button>
          </div>
          <div style={{ marginTop: "48px", display: "flex", gap: "32px", fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--fg-muted)" }}>
            <div><span style={{ color: "var(--fg-primary)", fontSize: "20px" }}>2,847</span><br/>practitioners</div>
            <div><span style={{ color: "var(--fg-primary)", fontSize: "20px" }}>14,209</span><br/>frames</div>
            <div><span style={{ color: "var(--fg-primary)", fontSize: "20px" }}>11,420 h</span><br/>integration</div>
          </div>
        </div>
        <div style={{ position: "relative", height: "560px" }}>
          <Photo photo={PHOTOS[7]} style={{ position: "absolute", inset: 0, height: "100%" }}/>
          <div style={{
            position: "absolute", left: 16, bottom: 16,
            background: "rgba(12,10,8,.78)", padding: "10px 14px",
            border: "1px solid var(--border-default)",
            fontFamily: "var(--font-mono)", fontSize: "11px",
            letterSpacing: "0.04em",
          }}>
            <div style={{ color: "var(--accent)" }}>FRAME OF THE WEEK</div>
            <div style={{ color: "var(--fg-primary)", marginTop: 4 }}>NGC 7000 · 18h SHO</div>
            <div style={{ color: "var(--fg-muted)" }}>Marie Dubois · Bortle 4</div>
          </div>
          <div style={{
            position: "absolute", top: 0, right: 0, width: 24, height: 24,
            borderTop: "1px solid var(--accent)", borderRight: "1px solid var(--accent)",
          }}/>
          <div style={{
            position: "absolute", bottom: 0, left: 0, width: 24, height: 24,
            borderBottom: "1px solid var(--accent)", borderLeft: "1px solid var(--accent)",
          }}/>
        </div>
      </section>

      {/* Filter bar */}
      <section style={{
        padding: "24px 64px",
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        borderBottom: "1px solid var(--border-subtle)",
      }}>
        <div style={{ display: "flex", gap: "8px" }}>
          {["All", "Galaxies", "Nebulae", "Solar System", "Wide field", "Lunar"].map((c, i) => (
            <button key={c} className={"chip " + (i === 0 ? "chip-accent" : "")} style={{ height: 28, padding: "0 12px" }}>{c}</button>
          ))}
        </div>
        <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
          <span className="t-label">SORT</span>
          <button className="chip">Newest first ▾</button>
          <span className="t-label" style={{ marginLeft: 12 }}>VIEW</span>
          <div style={{ display: "flex", border: "1px solid var(--border-default)" }}>
            <button style={{ width: 32, height: 28, background: "var(--bg-elevated)", color: "var(--accent)" }}>▦</button>
            <button style={{ width: 32, height: 28, color: "var(--fg-muted)" }}>≡</button>
          </div>
        </div>
      </section>

      {/* Masonry-ish grid */}
      <section style={{ padding: "32px 64px" }}>
        <div style={{
          columnCount: 3,
          columnGap: "20px",
        }}>
          {PHOTOS.slice(0, 12).map((p, i) => {
            const heights = [320, 480, 380, 280, 540, 320, 420, 380, 340, 460, 300, 400];
            return (
              <div key={i} style={{ breakInside: "avoid", marginBottom: "20px" }}>
                <div style={{ position: "relative", height: heights[i] }}>
                  <Photo photo={p} style={{ position: "absolute", inset: 0, height: "100%" }}/>
                </div>
                <div style={{ display: "flex", justifyContent: "space-between", padding: "10px 2px", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--fg-muted)" }}>
                  <span style={{ color: "var(--fg-primary)" }}>{p.target}</span>
                  <span>{p.integration}</span>
                </div>
                <div style={{ padding: "0 2px", fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--fg-faint)", letterSpacing: "0.04em" }}>
                  {p.photographer.toUpperCase()}
                </div>
              </div>
            );
          })}
        </div>
      </section>

      <div style={{ display: "flex", justifyContent: "center", padding: "0 0 64px" }}>
        <button className="btn btn-secondary btn-lg">Load page 2 of 974</button>
      </div>

      <AppFooter/>
    </div>
  );
};

/* ============================================================
   2. PHOTO DETAIL — desktop with EXIF panel
   ============================================================ */
window.ScreenPhotoDetail = function ScreenPhotoDetail({ marks, expanded = true }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  const photo = PHOTOS[7]; // NGC 7000
  return (
    <div className="screen" style={{ width: "1440px", height: "1100px", overflow: "hidden" }}>
      <AppHeader active="Gallery" marks={marks}/>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 380px", height: "calc(100% - 64px)" }}>
        {/* Image stage */}
        <div style={{ background: "#000", position: "relative", display: "flex", alignItems: "center", justifyContent: "center", padding: "48px" }}>
          <div style={{ position: "relative", width: "100%", height: "100%", maxHeight: "780px" }}>
            <Photo photo={photo} style={{ position: "absolute", inset: 0 }}/>
            {/* corner reticles */}
            {[[0,0],[1,0],[0,1],[1,1]].map(([x,y],i)=>(
              <div key={i} style={{
                position:"absolute",
                [x?"right":"left"]: -8,
                [y?"bottom":"top"]: -8,
                width: 14, height: 14,
                [`border${y?"Bottom":"Top"}`]: "1px solid var(--accent)",
                [`border${x?"Right":"Left"}`]: "1px solid var(--accent)",
              }}/>
            ))}
          </div>
          {/* zoom controls */}
          <div style={{ position: "absolute", left: 24, bottom: 24, display: "flex", gap: 4 }}>
            {["100%", "fit", "+", "−"].map(b => (
              <button key={b} style={{
                width: 32, height: 32,
                background: "rgba(12,10,8,.7)",
                border: "1px solid var(--border-default)",
                color: "var(--fg-secondary)",
                fontFamily: "var(--font-mono)", fontSize: 11,
              }}>{b}</button>
            ))}
          </div>
        </div>

        {/* Info panel */}
        <aside style={{
          background: "var(--bg-base)",
          borderLeft: "1px solid var(--border-subtle)",
          overflow: "auto",
        }}>
          <div style={{ padding: "32px" }}>
            <div className="t-eyebrow" style={{ marginBottom: 12, color: "var(--accent)" }}>● PUBLISHED 17 MAR 2026</div>
            <h1 style={{
              fontFamily: "var(--font-display)",
              fontSize: 32,
              fontWeight: 400,
              margin: 0,
              lineHeight: 1.1,
            }}>
              <em>NGC 7000</em><br/>North America Nebula
            </h1>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginTop: 16, paddingTop: 16, borderTop: "1px solid var(--border-subtle)" }}>
              <div style={{ width: 36, height: 36, borderRadius: "50%", background: "var(--accent)", color: "var(--accent-ink)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--font-display)", fontSize: 16 }}>M</div>
              <div>
                <div style={{ fontWeight: 500 }}>Marie Dubois</div>
                <div className="t-meta">42 frames · Bortle 4 · Provence</div>
              </div>
              <button className="btn btn-secondary btn-sm" style={{ marginLeft: "auto" }}>Follow</button>
            </div>

            <p style={{ marginTop: 24, fontSize: 14, lineHeight: 1.65, color: "var(--fg-secondary)" }}>
              North America Nebula in narrowband, 18 h total integration over 4 nights from a Bortle 4 site in Provence.
              Hubble palette (SHO), processed in PixInsight with a careful background-extraction pass and a
              non-linear stretch designed to preserve the dim H-α tendrils running through Pelican.
            </p>

            <div style={{ display: "flex", gap: 8, marginTop: 24 }}>
              <button className="btn btn-secondary btn-sm">♡ 248 appreciations</button>
              <button className="btn btn-ghost btn-sm">12 comments</button>
              <button className="btn btn-ghost btn-sm" style={{ marginLeft: "auto" }}>↗ Share</button>
            </div>
          </div>

          {/* EXIF panel */}
          <div style={{ padding: "0 32px 32px" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "16px 0", borderTop: "1px solid var(--border-default)", borderBottom: "1px solid var(--border-subtle)" }}>
              <span className="t-label" style={{ color: "var(--fg-primary)", letterSpacing: "0.16em" }}>ACQUISITION RECORD</span>
              <span className="t-meta">{expanded ? "▾" : "▸"}</span>
            </div>
            {expanded && (
              <table className="exif" style={{ marginTop: 8 }}>
                <tbody>
                  <tr><th>Target</th><td>NGC 7000<br/><span style={{color:"var(--fg-muted)"}}>North America Nebula</span></td></tr>
                  <tr><th>Captured</th><td>14–17 Mar 2026<br/>4 sessions</td></tr>
                  <tr><th>Camera</th><td>ZWO ASI2600MC Pro<br/><span style={{color:"var(--fg-muted)"}}>Cooled CMOS, −10 °C</span></td></tr>
                  <tr><th>Telescope</th><td>Takahashi FSQ-106EDX4<br/><span style={{color:"var(--fg-muted)"}}>f/5, 530 mm</span></td></tr>
                  <tr><th>Mount</th><td>10Micron GM1000 HPS</td></tr>
                  <tr><th>Filters</th><td>Antlia 3 nm SHO</td></tr>
                  <tr><th>Exposure</th><td>180 × 360 s<br/><span style={{color:"var(--accent)"}}>= 18.0 hours</span></td></tr>
                  <tr><th>Gain</th><td>100</td></tr>
                  <tr><th>RA / Dec</th><td>20ʰ 58ᵐ 47ˢ<br/>+44° 19′ 53″</td></tr>
                  <tr><th>Field</th><td>1.7° × 1.1°</td></tr>
                  <tr><th>Pixel scale</th><td>1.92 ″/px</td></tr>
                </tbody>
              </table>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
};

/* ============================================================
   3. PHOTO DETAIL — mobile, 390 wide
   ============================================================ */
window.ScreenPhotoMobile = function ScreenPhotoMobile({ marks }) {
  const Photo = window.Photo;
  const photo = PHOTOS[7];
  const Wordmark = marks.Wordmark;
  return (
    <div className="screen" style={{ width: "390px", height: "844px", overflow: "hidden", display: "flex", flexDirection: "column" }}>
      {/* tiny header */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "14px 16px", borderBottom: "1px solid var(--border-subtle)" }}>
        <button style={{ color: "var(--fg-secondary)" }}>←</button>
        <Wordmark size={16} italic={true}>Astrophoto</Wordmark>
        <button style={{ color: "var(--fg-secondary)" }}>⋯</button>
      </div>
      <div style={{ position: "relative", width: "100%", height: "320px", background: "#000" }}>
        <Photo photo={photo} style={{ position: "absolute", inset: 0 }}/>
      </div>
      <div style={{ padding: "20px 20px 0", flex: 1, overflow: "auto" }}>
        <div className="t-eyebrow" style={{ color: "var(--accent)", marginBottom: 8 }}>● 17 MAR 2026</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 28, fontWeight: 400, margin: 0, lineHeight: 1.05 }}>
          <em>NGC 7000</em><br/>North America Nebula
        </h1>
        <div style={{ display: "flex", alignItems: "center", gap: 10, margin: "16px 0", paddingBottom: 16, borderBottom: "1px solid var(--border-subtle)" }}>
          <div style={{ width: 28, height: 28, borderRadius: "50%", background: "var(--accent)", color: "var(--accent-ink)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--font-display)", fontSize: 13 }}>M</div>
          <div style={{fontSize:13}}>Marie Dubois</div>
          <button className="btn btn-secondary btn-sm" style={{ marginLeft: "auto", height: 26, padding: "0 10px" }}>Follow</button>
        </div>
        <p style={{ fontSize: 13, lineHeight: 1.6, color: "var(--fg-secondary)" }}>
          18 h total integration, narrowband SHO, Bortle 4. Processed in PixInsight.
        </p>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "12px 0", borderTop: "1px solid var(--border-default)", marginTop: 12 }}>
          <span className="t-label" style={{ color: "var(--fg-primary)" }}>ACQUISITION</span>
          <span className="t-meta">▾</span>
        </div>
        <table className="exif" style={{ fontSize: 11 }}>
          <tbody>
            <tr><th>Camera</th><td>ASI2600MC Pro</td></tr>
            <tr><th>Scope</th><td>Tak FSQ-106EDX4</td></tr>
            <tr><th>Exposure</th><td>180 × 360 s = 18 h</td></tr>
            <tr><th>Filters</th><td>Antlia 3 nm SHO</td></tr>
            <tr><th>RA/Dec</th><td>20ʰ58ᵐ / +44°19′</td></tr>
          </tbody>
        </table>
      </div>
      {/* sticky bottom action bar */}
      <div style={{ display: "flex", borderTop: "1px solid var(--border-subtle)", background: "var(--bg-base)" }}>
        <button style={{ flex: 1, padding: "16px 0", color: "var(--accent)", fontFamily: "var(--font-mono)", fontSize: 12 }}>♡ 248</button>
        <button style={{ flex: 1, padding: "16px 0", color: "var(--fg-secondary)", borderLeft: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: 12 }}>💬 12</button>
        <button style={{ flex: 1, padding: "16px 0", color: "var(--fg-secondary)", borderLeft: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: 12 }}>↗ Share</button>
      </div>
    </div>
  );
};

/* ============================================================
   4. USER PROFILE
   ============================================================ */
window.ScreenProfile = function ScreenProfile({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1500px", overflow: "hidden" }}>
      <AppHeader marks={marks}/>
      {/* hero */}
      <section style={{ padding: "64px 64px 32px", display: "grid", gridTemplateColumns: "120px 1fr auto", gap: "32px", alignItems: "start", borderBottom: "1px solid var(--border-subtle)" }}>
        <div style={{ width: 120, height: 120, borderRadius: "50%", background: "var(--accent)", color: "var(--accent-ink)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--font-display)", fontSize: 56 }}>M</div>
        <div>
          <div className="t-eyebrow" style={{ marginBottom: 8 }}>PRACTITIONER · MEMBER SINCE 2026</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 64, fontWeight: 400, margin: 0, lineHeight: 1 }}>
            Marie <em>Dubois</em>
          </h1>
          <p style={{ marginTop: 16, fontSize: 15, color: "var(--fg-secondary)", maxWidth: 640 }}>
            Deep-sky narrowband imaging from a Bortle 4 site in Haute-Provence.
            Mostly emission nebulae and galaxy clusters. Always happy to share processing notes.
          </p>
          <div style={{ display: "flex", gap: 24, marginTop: 24, fontFamily: "var(--font-mono)", fontSize: 12 }}>
            <div><span style={{ color: "var(--fg-primary)", fontSize: 22 }}>42</span><br/><span style={{color:"var(--fg-muted)"}}>frames</span></div>
            <div><span style={{ color: "var(--fg-primary)", fontSize: 22 }}>318 h</span><br/><span style={{color:"var(--fg-muted)"}}>integration</span></div>
            <div><span style={{ color: "var(--fg-primary)", fontSize: 22 }}>1,204</span><br/><span style={{color:"var(--fg-muted)"}}>followers</span></div>
            <div><span style={{ color: "var(--fg-primary)", fontSize: 22 }}>8</span><br/><span style={{color:"var(--fg-muted)"}}>collections</span></div>
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          <button className="btn btn-primary">Follow</button>
          <button className="btn btn-secondary">Message</button>
          <div className="t-meta" style={{ marginTop: 8, textAlign: "right" }}>
            44.1°N · 6.2°E<br/>Bortle 4 · SQM 21.8
          </div>
        </div>
      </section>

      {/* equipment strip */}
      <section style={{ padding: "20px 64px", borderBottom: "1px solid var(--border-subtle)", display: "flex", gap: 32, fontFamily: "var(--font-mono)", fontSize: 12 }}>
        <div><span style={{color:"var(--fg-muted)"}}>SCOPE</span> &nbsp; Tak FSQ-106EDX4</div>
        <div><span style={{color:"var(--fg-muted)"}}>CAM</span> &nbsp; ZWO ASI2600MC Pro</div>
        <div><span style={{color:"var(--fg-muted)"}}>MOUNT</span> &nbsp; 10Micron GM1000 HPS</div>
        <div><span style={{color:"var(--fg-muted)"}}>FILTERS</span> &nbsp; Antlia 3nm SHO</div>
      </section>

      {/* tabs */}
      <section style={{ padding: "0 64px", borderBottom: "1px solid var(--border-subtle)", display: "flex", gap: 40 }}>
        {["Frames · 42", "Collections · 8", "Equipment", "About"].map((t,i)=>(
          <a key={t} className={"nav-link" + (i===0?" active":"")} style={{ padding: "20px 0" }}>{t}</a>
        ))}
        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 12 }}>
          <span className="t-label">SORT</span><button className="chip">Newest ▾</button>
        </div>
      </section>

      {/* grid */}
      <section style={{ padding: "32px 64px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 16 }}>
          {PHOTOS.slice(0, 8).map((p,i)=>(
            <div key={i}>
              <div style={{ position: "relative", aspectRatio: "1 / 1" }}>
                <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", padding: "8px 2px", fontFamily: "var(--font-mono)", fontSize: 11 }}>
                <span style={{ color: "var(--fg-primary)" }}>{p.target}</span>
                <span style={{ color: "var(--fg-muted)" }}>{p.integration}</span>
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};
