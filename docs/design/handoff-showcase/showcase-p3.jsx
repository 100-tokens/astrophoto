/* =============================================================
   ASTROPHOTO · SHOWCASE PHASE 3 — Discovery
   /explore · /t/<slug> · /equip/<kind>/<slug> · /search
   ============================================================= */

const PHOTOS_D = window.PHOTOS;
const Eyebrow3 = window.AP_Eyebrow;
const Display3 = window.AP_Display;
const PageHeader3 = window.AP_PageHeader;
const Tile3 = window.AP_Tile;

/* ============================================================
   3A — EXPLORE FEED  /explore
   ============================================================ */
window.ScreenExplore = function ({ marks }) {
  const AppHeader = window.AppHeader;
  return (
    <div className="screen" style={{ width: 1440, height: 1700, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <PageHeader3
        eyebrow="● EXPLORE · 12,418 PUBLISHED FRAMES · UPDATED LIVE"
        title={<>The <em>archive</em>, across photographers</>}
        right={
          <div style={{ textAlign: "right" }}>
            <div className="t-meta" style={{ color: "var(--fg-muted)" }}>NEW MOON IN 6 DAYS</div>
            <div className="t-meta" style={{ color: "var(--accent)", marginTop: 6 }}>● 47 NEW IN THE LAST 24 HRS</div>
          </div>
        }
      />

      {/* Filter rail */}
      <section style={{
        padding: "20px 64px", borderBottom: "1px solid var(--border-subtle)",
        display: "flex", justifyContent: "space-between", alignItems: "center", gap: 16,
      }}>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {["Newest", "Most appreciated", "Most discussed"].map((s, i) => (
            <button key={s} className={"chip " + (i === 0 ? "chip-accent" : "")}>{s}</button>
          ))}
          <span style={{ width: 1, background: "var(--border-default)", margin: "0 4px" }}/>
          {["24 h", "7 d", "30 d", "All time"].map((s, i) => (
            <button key={s} className={"chip " + (i === 1 ? "chip-accent" : "")}>{s}</button>
          ))}
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          {["DSO", "Planetary", "Lunar", "Solar", "Wide-field", "Nightscape"].map((c, i) => (
            <button key={c} className={"chip " + (i === 0 ? "chip-accent" : "")}>{c}</button>
          ))}
          <span style={{ width: 1, background: "var(--border-default)", margin: "0 4px" }}/>
          <button className="chip">✓ Following only</button>
          <button className="chip" style={{ color: "var(--fg-faint)" }}>✕ Clear</button>
        </div>
      </section>

      {/* Grid */}
      <section style={{ padding: "24px 64px" }}>
        {/* row 1 */}
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.5} h={240} likes={412} when="2 h ago" handle="marie-dubois"/>
          <CrossAuthorTile photo={PHOTOS_D[2]} flex={1.4} h={240} likes={134} when="5 h ago" handle="starhunter42"/>
          <CrossAuthorTile photo={PHOTOS_D[3]} flex={1.5} h={240} likes={203} when="6 h ago" handle="k-aalto"/>
          <CrossAuthorTile photo={PHOTOS_D[7]} flex={1.4} h={240} likes={87}  when="8 h ago" handle="marie-dubois"/>
        </div>
        {/* row 2 */}
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[4]} flex={1.7} h={240} likes={156} when="11 h ago" handle="jwst-lover"/>
          <CrossAuthorTile photo={PHOTOS_D[5]} flex={1.3} h={240} likes={203} when="14 h ago" handle="l-petrov"/>
          <CrossAuthorTile photo={PHOTOS_D[8]} flex={1.5} h={240} likes={62}  when="17 h ago" handle="p-halverson"/>
          <CrossAuthorTile photo={PHOTOS_D[9]} flex={1.0} h={240} likes={311} when="22 h ago" handle="s-tanaka"/>
        </div>
        {/* row 3 — wide panorama */}
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[10]} flex={2.5} h={240} likes={198} when="1 d ago" handle="a-dimov"/>
          <CrossAuthorTile photo={PHOTOS_D[11]} flex={1.0} h={240} likes={78}  when="1 d ago" handle="r-mehta"/>
          <CrossAuthorTile photo={PHOTOS_D[12]} flex={1.4} h={240} likes={156} when="1 d ago" handle="l-petrov"/>
        </div>
        {/* row 4 */}
        <div style={{ display: "flex", gap: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[13]} flex={1.0} h={240} likes={412} when="2 d ago" handle="l-viatour"/>
          <CrossAuthorTile photo={PHOTOS_D[14]} flex={1.5} h={240} likes={203} when="2 d ago" handle="southern-sky"/>
          <CrossAuthorTile photo={PHOTOS_D[6]}  flex={1.0} h={240} likes={87}  when="2 d ago" handle="cometchaser"/>
          <CrossAuthorTile photo={PHOTOS_D[0]}  flex={1.16} h={240} likes={511} when="2 d ago" handle="hubble"/>
        </div>

        <div style={{ marginTop: 32, textAlign: "center" }}>
          <button className="btn btn-secondary btn-lg">Load more · cursor (newest, id) ↓</button>
          <div className="t-meta" style={{ marginTop: 12, color: "var(--fg-muted)" }}>
            CURSOR · (PUBLISHED_AT, ID) &lt; (2026-05-01T18:14, F4A7…) · INFINITE SCROLL ON DESKTOP
          </div>
        </div>
      </section>
    </div>
  );
};

function CrossAuthorTile({ photo, flex, h, likes, when, handle }) {
  const Photo = window.Photo;
  return (
    <div style={{ position: "relative", flex, height: h }}>
      <Photo photo={photo} style={{ position: "absolute", inset: 0 }}/>
      <div style={{
        position: "absolute", left: 0, right: 0, bottom: 0, padding: "10px 10px 8px",
        background: "linear-gradient(to top, rgba(0,0,0,.78), transparent 90%)",
      }}>
        <div style={{
          display: "flex", justifyContent: "space-between", alignItems: "end", gap: 8,
        }}>
          <div style={{ minWidth: 0 }}>
            <div style={{
              fontFamily: "var(--font-display)", fontSize: 14, fontStyle: "italic",
              color: "#fff", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
            }}>{photo.target.split("·")[0].trim()}</div>
            <div className="t-meta" style={{ color: "rgba(255,255,255,.65)", marginTop: 2 }}>
              @{handle.toUpperCase()} · {when.toUpperCase()}
            </div>
          </div>
          <div className="chip" style={{
            background: "rgba(12,10,8,.7)", color: "var(--accent)", borderColor: "var(--accent-dim)",
          }}>♡ {likes}</div>
        </div>
      </div>
    </div>
  );
}
window.AP_CrossAuthorTile = CrossAuthorTile;

/* ============================================================
   3B — TARGET PAGE  /t/m31
   ============================================================ */
window.ScreenTargetPage = function ({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: 1440, height: 1500, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>

      {/* Hero target header */}
      <section style={{
        padding: "40px 64px", borderBottom: "1px solid var(--border-subtle)",
        display: "grid", gridTemplateColumns: "1.4fr 1fr", gap: 64, alignItems: "end",
      }}>
        <div>
          <Eyebrow3>● TARGET · MESSIER OBJECT</Eyebrow3>
          <div style={{ display: "flex", alignItems: "baseline", gap: 24, marginTop: 8 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 64, color: "var(--accent)", letterSpacing: "0.04em" }}>M31</span>
            <Display3 size={56} style={{ margin: 0 }}>Andromeda <em>Galaxy</em></Display3>
          </div>
          <div style={{ marginTop: 16, display: "flex", gap: 8, flexWrap: "wrap" }}>
            <span className="chip">Also known as</span>
            {["NGC 224", "UGC 454", "Great Andromeda Nebula"].map(a => (
              <span key={a} className="chip" style={{ borderColor: "var(--accent-dim)", color: "var(--accent)" }}>{a}</span>
            ))}
          </div>
          <p style={{ color: "var(--fg-secondary)", fontSize: 14, lineHeight: 1.7, marginTop: 20, maxWidth: 640 }}>
            Spiral galaxy in Andromeda · 2.537 million light-years · the nearest large galaxy to the Milky Way.
            <span className="t-meta" style={{ color: "var(--fg-muted)", marginLeft: 8 }}>SOURCE · WIKIPEDIA</span>
          </p>
        </div>
        <div style={{
          padding: 24, border: "1px solid var(--border-default)", background: "var(--bg-base)",
          display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16,
        }}>
          <Stat n="482" l="published frames" accent/>
          <Stat n="213" l="contributors"/>
          <Stat n="1,206 h" l="combined integration"/>
          <Stat n="14 / 9" l="bortle range observed"/>
          <div style={{ gridColumn: "1 / -1" }} className="t-meta">
            ● THIS PAGE INCLUDES ALL FRAMES TAGGED <span style={{ color: "var(--accent)" }}>M31</span>, NGC 224, OR UGC 454.
            FUTURE PLATE-SOLVED MATCHES WILL APPEAR HERE AUTOMATICALLY.
          </div>
        </div>
      </section>

      {/* Filters */}
      <section style={{
        padding: "20px 64px", borderBottom: "1px solid var(--border-subtle)",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <div style={{ display: "flex", gap: 8 }}>
          {["Most appreciated", "Newest", "Longest integration"].map((s, i) => (
            <button key={s} className={"chip " + (i === 0 ? "chip-accent" : "")}>{s}</button>
          ))}
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          {["Any equipment ▾", "Any photographer ▾"].map(s => (
            <button key={s} className="chip">{s}</button>
          ))}
        </div>
      </section>

      <section style={{ padding: "24px 64px" }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.5} h={260} likes={1402} when="2 d" handle="marie-dubois"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.0} h={260} likes={892}  when="5 d" handle="m31-fanatic"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.5} h={260} likes={734}  when="6 d" handle="r-mehta"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.0} h={260} likes={511}  when="9 d" handle="p-halverson"/>
        </div>
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.7} h={260} likes={487} when="14 d" handle="l-petrov"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.3} h={260} likes={356} when="18 d" handle="s-tanaka"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.5} h={260} likes={278} when="21 d" handle="a-dimov"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.0} h={260} likes={193} when="1 mo" handle="cometchaser"/>
        </div>
      </section>
    </div>
  );
};

function Stat({ n, l, accent }) {
  return (
    <div>
      <div style={{
        fontFamily: "var(--font-display)", fontSize: 32, lineHeight: 1,
        color: accent ? "var(--accent)" : "var(--fg-primary)",
      }}>{n}</div>
      <div className="t-meta" style={{ marginTop: 6 }}>{l.toUpperCase()}</div>
    </div>
  );
}

/* ============================================================
   3C — EQUIPMENT PAGE  /equip/camera/zwo-asi2600mc-pro
   ============================================================ */
window.ScreenEquipmentPage = function ({ marks }) {
  const AppHeader = window.AppHeader;
  return (
    <div className="screen" style={{ width: 1440, height: 1300, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>

      <section style={{
        padding: "40px 64px", borderBottom: "1px solid var(--border-subtle)",
        display: "grid", gridTemplateColumns: "1fr auto", gap: 32, alignItems: "end",
      }}>
        <div>
          <Eyebrow3>● EQUIPMENT · CAMERA · /EQUIP/CAMERA/ZWO-ASI2600MC-PRO</Eyebrow3>
          <Display3 size={48} style={{ marginTop: 8 }}>ZWO <em>ASI2600MC Pro</em></Display3>
          <div style={{ marginTop: 16, display: "flex", gap: 12, flexWrap: "wrap" }}>
            <span className="chip" style={{ borderColor: "var(--accent-dim)", color: "var(--accent)" }}>OSC · One-shot color</span>
            <span className="chip">Sony IMX571 · 26 MP · APS-C</span>
            <span className="chip">Cooled · ΔT-35°C</span>
          </div>
        </div>
        <div style={{ display: "flex", gap: 32 }}>
          <Stat n="138" l="frames"/>
          <Stat n="64" l="photographers"/>
          <Stat n="412 h" l="integration" accent/>
        </div>
      </section>

      <section style={{
        padding: "20px 64px", borderBottom: "1px solid var(--border-subtle)",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <div style={{ display: "flex", gap: 8 }}>
          {["Newest", "Most appreciated"].map((s, i) => (
            <button key={s} className={"chip " + (i === 0 ? "chip-accent" : "")}>{s}</button>
          ))}
        </div>
        <div className="t-meta" style={{ color: "var(--fg-muted)" }}>
          JOIN · LOWER(PHOTOS.CAMERA) = "ZWO-ASI2600MC-PRO"
        </div>
      </section>

      <section style={{ padding: "24px 64px" }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[7]} flex={1.5} h={260} likes={487} when="2 d" handle="marie-dubois"/>
          <CrossAuthorTile photo={PHOTOS_D[1]} flex={1.0} h={260} likes={356} when="5 d" handle="r-mehta"/>
          <CrossAuthorTile photo={PHOTOS_D[2]} flex={1.5} h={260} likes={278} when="6 d" handle="cometchaser"/>
          <CrossAuthorTile photo={PHOTOS_D[6]} flex={1.0} h={260} likes={193} when="9 d" handle="comet-chaser"/>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <CrossAuthorTile photo={PHOTOS_D[8]} flex={1.0} h={260} likes={156} when="14 d" handle="p-halverson"/>
          <CrossAuthorTile photo={PHOTOS_D[11]} flex={1.5} h={260} likes={134} when="18 d" handle="r-mehta"/>
          <CrossAuthorTile photo={PHOTOS_D[14]} flex={1.5} h={260} likes={92} when="21 d" handle="southern-sky"/>
          <CrossAuthorTile photo={PHOTOS_D[5]}  flex={1.0} h={260} likes={67} when="1 mo" handle="l-petrov"/>
        </div>
      </section>

      {/* Related equipment */}
      <section style={{ padding: "32px 64px 0" }}>
        <Eyebrow3>OFTEN PAIRED WITH</Eyebrow3>
        <div style={{ marginTop: 16, display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 16 }}>
          {[
            ["TELESCOPE", "Takahashi FSQ-106EDX4", "47"],
            ["MOUNT",     "10Micron GM1000 HPS",   "32"],
            ["FILTERS",   "Antlia 3 nm SHO",       "28"],
            ["GUIDING",   "ASI120MM Mini · OAG",   "61"],
          ].map(([k, v, n]) => (
            <a key={k} style={{
              padding: 16, border: "1px solid var(--border-subtle)", background: "var(--bg-raised)",
            }}>
              <div className="t-label">{k}</div>
              <div style={{
                fontFamily: "var(--font-display)", fontSize: 16, fontStyle: "italic", marginTop: 6,
              }}>{v}</div>
              <div className="t-meta" style={{ marginTop: 8, color: "var(--accent)" }}>● {n} CO-OCCURRENCES</div>
            </a>
          ))}
        </div>
      </section>
    </div>
  );
};

/* ============================================================
   3D — SEARCH RESULTS  /search?q=andromeda
   ============================================================ */
window.ScreenSearch = function ({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: 1440, height: 1500, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>

      {/* Search bar emphasised */}
      <section style={{ padding: "32px 64px 0" }}>
        <Eyebrow3>SEARCH</Eyebrow3>
        <div style={{
          marginTop: 12, display: "flex", alignItems: "center", gap: 12,
          padding: "0 16px", height: 56,
          border: "1px solid var(--accent)", background: "var(--bg-base)",
        }}>
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="var(--accent)" strokeWidth="1.4">
            <circle cx="7" cy="7" r="5"/><line x1="11" y1="11" x2="14" y2="14"/>
          </svg>
          <input style={{
            flex: 1, height: "100%", background: "transparent", border: 0, outline: "none",
            fontFamily: "var(--font-display)", fontStyle: "italic", fontSize: 22,
            color: "var(--fg-primary)",
          }} defaultValue="andromeda"/>
          <span className="t-meta" style={{ color: "var(--fg-muted)" }}>134 RESULTS · 12 MS</span>
          <span style={{ color: "var(--fg-faint)" }}>✕</span>
        </div>
      </section>

      <section style={{ padding: "24px 64px 0", display: "grid", gridTemplateColumns: "320px 1fr", gap: 48 }}>
        {/* Left rail */}
        <aside>
          <div className="t-label" style={{ marginBottom: 12 }}>FILTER BY</div>
          {[
            ["Targets",       "5"],
            ["Photographers", "8"],
            ["Photos",        "121"],
            ["Tags",          "3"],
          ].map(([k, n], i) => (
            <div key={k} style={{
              padding: "10px 12px", display: "flex", justifyContent: "space-between",
              border: "1px solid " + (i === 0 ? "var(--accent)" : "var(--border-subtle)"),
              background: i === 0 ? "var(--bg-accent-tint)" : "var(--bg-raised)",
              color: i === 0 ? "var(--accent)" : "var(--fg-secondary)",
              marginBottom: 6, fontFamily: "var(--font-mono)", fontSize: 12,
            }}>
              <span>{k}</span><span>{n}</span>
            </div>
          ))}

          <div className="t-meta" style={{ marginTop: 24, color: "var(--fg-muted)", lineHeight: 1.7 }}>
            ● ILIKE ACROSS HANDLES, DISPLAY NAMES, TARGETS.CANONICAL_NAME, ALIASES, TAGS, PHOTOS.TARGET, CAPTIONS.
            CAP 5 / 5 / 24 PER GROUP.
          </div>
        </aside>

        {/* Results */}
        <div>
          {/* Targets */}
          <ResultsSection label="TARGETS · 5">
            {[
              ["M31", "Andromeda Galaxy", "messier · NGC 224 · UGC 454", "482"],
              ["M32", "Le Gentil",        "messier · companion of M31",   "87"],
              ["M110","Andromeda's dwarf","messier · NGC 205",            "62"],
              ["NGC 891","Silver Sliver", "ngc · andromeda constellation","14"],
              ["IC 10","Starburst",       "ic · andromeda constellation",  "7"],
            ].map(([slug, name, kind, n]) => (
              <div key={slug} style={{
                display: "grid", gridTemplateColumns: "100px 1fr auto",
                gap: 16, padding: "12px 0",
                borderBottom: "1px dashed var(--border-subtle)", alignItems: "center",
              }}>
                <span style={{
                  fontFamily: "var(--font-mono)", fontSize: 14,
                  color: "var(--accent)",
                }}>{slug}</span>
                <div>
                  <div style={{ fontFamily: "var(--font-display)", fontSize: 17, fontStyle: "italic" }}>{name}</div>
                  <div className="t-meta" style={{ marginTop: 2 }}>{kind.toUpperCase()}</div>
                </div>
                <span className="chip" style={{ color: "var(--accent)" }}>♡ {n} photos</span>
              </div>
            ))}
          </ResultsSection>

          {/* Photographers */}
          <ResultsSection label="PHOTOGRAPHERS · 8" style={{ marginTop: 32 }}>
            {[
              ["andromeda_aficionado", "Marco Bianchi",   "418"],
              ["nightsky_andy",        "Andy Lefebvre",   "201"],
              ["andromeda-2024",       "Selma Castellan", "84"],
            ].map(([h, n, c]) => (
              <div key={h} style={{
                display: "grid", gridTemplateColumns: "44px 1fr auto auto",
                gap: 14, padding: "12px 0",
                borderBottom: "1px dashed var(--border-subtle)", alignItems: "center",
              }}>
                <div style={{
                  width: 44, height: 44,
                  background: "var(--accent)", color: "var(--accent-ink)",
                  display: "flex", alignItems: "center", justifyContent: "center",
                  fontFamily: "var(--font-display)", fontSize: 18,
                }}>{n[0]}</div>
                <div>
                  <div style={{ fontFamily: "var(--font-display)", fontSize: 17, fontStyle: "italic" }}>{n}</div>
                  <div className="t-meta">@{h.toUpperCase()} · {c} FRAMES</div>
                </div>
                <button className="btn btn-secondary btn-sm">+ Follow</button>
                <span style={{ color: "var(--fg-faint)" }}>→</span>
              </div>
            ))}
          </ResultsSection>

          {/* Photos */}
          <ResultsSection label="PHOTOS · 121" style={{ marginTop: 32 }}>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8 }}>
              {[1, 1, 1, 1, 1, 1, 1, 1].map((idx, i) => (
                <div key={i} style={{ position: "relative", aspectRatio: "4/3" }}>
                  <Photo photo={PHOTOS_D[1]} style={{ position: "absolute", inset: 0 }}/>
                  <div className="t-meta" style={{
                    position: "absolute", left: 8, bottom: 8, color: "#fff",
                    background: "rgba(12,10,8,.7)", padding: "2px 6px",
                  }}>@{["MARIE","R-MEHTA","S-TANAKA","P-HALVERSON","COMETCHASER","M31-FANATIC","L-PETROV","A-DIMOV"][i]}</div>
                </div>
              ))}
            </div>
            <div style={{ marginTop: 12, textAlign: "center" }}>
              <button className="btn btn-secondary btn-sm">See all 121 photos →</button>
            </div>
          </ResultsSection>
        </div>
      </section>
    </div>
  );
};

function ResultsSection({ label, children, style }) {
  return (
    <div style={style}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: 8 }}>
        <Eyebrow3 accent>● {label}</Eyebrow3>
        <span className="t-meta" style={{ color: "var(--accent)" }}>SEE ALL →</span>
      </div>
      {children}
    </div>
  );
}

/* ============================================================
   3E — SEARCH BAR · WITH AUTOCOMPLETE (component card)
   ============================================================ */
window.ScreenSearchBar = function ({ marks }) {
  return (
    <div className="screen" style={{ width: 720, height: 600, padding: 32, overflow: "hidden" }}>
      <Eyebrow3>● COMPONENT · &lt;SEARCHBAR&gt; · NAVBAR</Eyebrow3>
      <Display3 size={28} style={{ marginTop: 8 }}>Combined <em>autocomplete</em></Display3>
      <p style={{ color: "var(--fg-muted)", fontSize: 13, marginTop: 8 }}>
        Powers <code className="t-mono" style={{ color: "var(--accent)" }}>⌘K</code> from any page.
        Three-bucket dropdown: targets, photographers, recent searches.
      </p>

      <div style={{ marginTop: 32, position: "relative" }}>
        <div style={{
          display: "flex", alignItems: "center", gap: 10,
          padding: "0 14px", height: 44,
          border: "1px solid var(--accent)", background: "var(--bg-base)",
          fontFamily: "var(--font-mono)", fontSize: 13,
        }}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--accent)" strokeWidth="1.4">
            <circle cx="7" cy="7" r="5"/><line x1="11" y1="11" x2="14" y2="14"/>
          </svg>
          <span style={{ flex: 1, color: "var(--fg-primary)" }}>m31</span>
          <span className="t-meta" style={{ color: "var(--fg-muted)" }}>⌘K</span>
        </div>

        <div style={{
          position: "absolute", left: 0, right: 0, top: 48,
          background: "var(--bg-elevated)", border: "1px solid var(--border-default)",
          boxShadow: "var(--shadow-lg)",
        }}>
          <Bucket label="TARGETS · 3">
            {[
              ["M31", "Andromeda Galaxy", "482 photos"],
              ["M32", "companion · Le Gentil", "87 photos"],
              ["M110", "andromeda's dwarf", "62 photos"],
            ].map(([s, n, c], i) => (
              <DropRow key={s} sel={i === 0}>
                <span style={{ fontFamily: "var(--font-mono)", color: i === 0 ? "var(--accent)" : "var(--fg-secondary)", minWidth: 56 }}>{s}</span>
                <span style={{ fontFamily: "var(--font-display)", fontStyle: "italic", flex: 1 }}>{n}</span>
                <span className="t-meta" style={{ color: "var(--fg-muted)" }}>{c.toUpperCase()}</span>
              </DropRow>
            ))}
          </Bucket>
          <Bucket label="PHOTOGRAPHERS · 2">
            {[
              ["andromeda_aficionado", "Marco Bianchi"],
              ["m31-fanatic",          "Otto Reiter"],
            ].map(([h, n]) => (
              <DropRow key={h}>
                <div style={{ width: 24, height: 24, background: "var(--accent-dim)", color: "var(--accent-ink)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--font-display)", fontSize: 12 }}>{n[0]}</div>
                <span style={{ fontFamily: "var(--font-display)", fontStyle: "italic", flex: 1 }}>{n}</span>
                <span className="t-meta" style={{ color: "var(--fg-muted)" }}>@{h.toUpperCase()}</span>
              </DropRow>
            ))}
          </Bucket>
          <div style={{ padding: "8px 12px", display: "flex", justifyContent: "space-between" }} className="t-meta">
            <span style={{ color: "var(--fg-muted)" }}>↑↓ NAVIGATE · ↩ OPEN · ESC CLOSE</span>
            <span style={{ color: "var(--accent)" }}>SEE ALL 134 →</span>
          </div>
        </div>
      </div>
    </div>
  );
};

function Bucket({ label, children }) {
  return (
    <div style={{ borderBottom: "1px dashed var(--border-default)", padding: "8px 0" }}>
      <div className="t-label" style={{ padding: "4px 12px", color: "var(--accent)" }}>● {label}</div>
      {children}
    </div>
  );
}
function DropRow({ children, sel }) {
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 10,
      padding: "8px 12px",
      background: sel ? "var(--bg-accent-tint)" : "transparent",
      borderLeft: sel ? "2px solid var(--accent)" : "2px solid transparent",
    }}>{children}</div>
  );
}

/* ============================================================
   3F — DISCOVERY EMPTY STATES (small comparison artboard)
   ============================================================ */
window.ScreenDiscoveryEmpty = function ({ marks }) {
  return (
    <div className="screen" style={{ width: 1440, height: 700, padding: "48px 64px", overflow: "hidden" }}>
      <Eyebrow3>● DISCOVERY · EMPTY-STATE COPY HOOKS</Eyebrow3>
      <Display3 size={32} style={{ marginTop: 8 }}>When the page has <em>nothing</em> yet</Display3>
      <p style={{ color: "var(--fg-muted)", fontSize: 13, marginTop: 8, maxWidth: 720 }}>
        Each discovery surface gets a tailored empty-state. Tone is invitational, not apologetic — every empty page is a chance to publish the first frame.
      </p>

      <div style={{
        marginTop: 32, display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16,
      }}>
        {[
          ["/explore",        "No photos yet — be the first to upload."],
          ["/t/m31",          "No M31 photos yet. Upload yours to start the page."],
          ["/tag/widefield",  "Nothing tagged 'widefield' yet."],
          ["/equip/camera/…", "No photos with this camera yet."],
          ["/c/lunar",        "No photos in this category yet."],
          ["/search?q=zzz",   "Nothing matched 'zzz' — try a target or handle."],
        ].map(([path, msg]) => (
          <div key={path} style={{
            padding: 24, border: "1px dashed var(--border-default)",
            background: "var(--bg-raised)",
          }}>
            <div className="t-meta" style={{ color: "var(--accent)" }}>● {path}</div>
            <div style={{
              marginTop: 10, fontFamily: "var(--font-display)", fontSize: 16,
              fontStyle: "italic", color: "var(--fg-primary)", lineHeight: 1.5,
            }}>"{msg}"</div>
            <button className="btn btn-secondary btn-sm" style={{ marginTop: 14 }}>+ Upload a frame</button>
          </div>
        ))}
      </div>
    </div>
  );
};
