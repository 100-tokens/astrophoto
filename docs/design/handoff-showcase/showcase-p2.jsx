/* =============================================================
   ASTROPHOTO · SHOWCASE PHASE 2 — Hero Page (/u/<handle>)
   Cover, identity, bio, equipment, location, featured, gallery.
   Plus profile editor, cover picker, lightbox.
   ============================================================= */

const PHOTOS_H = window.PHOTOS;

/* ---------- shared bits reused from p1 ---------- */
const Eyebrow2 = window.AP_Eyebrow;
const Display2 = window.AP_Display;

/* ============================================================
   2A — HERO PAGE · /u/marie-dubois  (visitor view)
   ============================================================ */
window.ScreenHeroPage = function ({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  const photos = PHOTOS_H;

  const featured = [0, 1, 7, 9, 10, 4].map(i => photos[i]);
  const gallery = [2, 3, 5, 6, 8, 11, 12, 13, 14, 15 % photos.length, 0, 1].map(i => photos[i % photos.length]);

  return (
    <div className="screen" style={{ width: 1440, height: 2400, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>

      {/* === Cover === */}
      <div style={{ position: "relative", height: 480, overflow: "hidden" }}>
        <Photo photo={photos[7]} style={{ position: "absolute", inset: 0 }}/>
        <div style={{
          position: "absolute", inset: 0,
          background: "linear-gradient(to bottom, rgba(12,10,8,.0) 30%, rgba(12,10,8,.55) 70%, var(--bg-canvas) 100%)",
        }}/>
        <div style={{
          position: "absolute", right: 64, top: 24,
          padding: "6px 10px",
          background: "rgba(12,10,8,.6)",
          border: "1px solid var(--border-default)",
          fontFamily: "var(--font-mono)", fontSize: 11,
          color: "var(--fg-muted)", letterSpacing: "0.1em",
          textTransform: "uppercase",
        }}>● COVER · NGC 7000 · 18H 00M · MARCH 2026</div>
      </div>

      {/* === Identity === */}
      <section style={{
        padding: "0 64px", marginTop: -80, position: "relative", zIndex: 2,
        display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 32, alignItems: "end",
      }}>
        <div style={{
          width: 144, height: 144,
          border: "4px solid var(--bg-canvas)",
          background: "var(--accent)", color: "var(--accent-ink)",
          display: "flex", alignItems: "center", justifyContent: "center",
          fontFamily: "var(--font-display)", fontSize: 64, fontStyle: "italic",
        }}>M</div>
        <div style={{ paddingBottom: 12 }}>
          <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● PHOTOGRAPHER · @MARIE-DUBOIS</div>
          <Display2 size={56} style={{ marginTop: 6 }}>Marie <em>Dubois</em></Display2>
          <p style={{
            fontFamily: "var(--font-display)", fontSize: 20, fontStyle: "italic",
            color: "var(--fg-secondary)", marginTop: 12, marginBottom: 0,
          }}>Hunting deep-sky from a Bortle 6 backyard in Lyon.</p>
          <div style={{ marginTop: 16, display: "flex", gap: 12 }}>
            {["𝕏 twitter", "Instagram", "Astrobin", "🌐 marie.photo"].map(s => (
              <span key={s} className="chip">{s}</span>
            ))}
          </div>
        </div>
        <div style={{ paddingBottom: 12, display: "flex", gap: 8 }}>
          <button className="btn btn-secondary">Share profile</button>
          <button className="btn btn-primary">+ Follow</button>
        </div>
      </section>

      {/* === About / Equipment / Location columns === */}
      <section style={{
        padding: "48px 64px 0",
        display: "grid", gridTemplateColumns: "1.4fr 1fr 1fr", gap: 48,
        borderBottom: "1px solid var(--border-subtle)",
      }}>
        <div>
          <div className="t-label">ABOUT</div>
          <div style={{ marginTop: 12, color: "var(--fg-secondary)", fontSize: 14, lineHeight: 1.7 }}>
            <p style={{ marginTop: 0 }}>
              I shoot from a small balcony in central Lyon, Bortle 6 with the city's light-dome
              eating the southern horizon. Mostly narrowband through the FSQ-106 — Hα and OIII
              are still the only signal I can pull cleanly through the orange.
            </p>
            <p>
              I publish only what survives at least <em>six clear hours</em> of integration.
              Anything less goes into the drafts pile and waits for next season.
            </p>
            <a className="t-meta" style={{ color: "var(--accent)" }}>READ MORE ↓</a>
          </div>
        </div>
        <div>
          <div className="t-label">EQUIPMENT · DEFAULT LOADOUT</div>
          <div style={{ marginTop: 12, fontFamily: "var(--font-mono)", fontSize: 13 }}>
            <EqRow k="SCOPE"     v="Takahashi FSQ-106EDX4"/>
            <EqRow k="CAMERA"    v="ZWO ASI2600MC Pro"/>
            <EqRow k="MOUNT"     v="10Micron GM1000 HPS"/>
            <EqRow k="FILTERS"   v="Antlia 3 nm SHO"/>
            <EqRow k="GUIDING"   v="ASI120MM Mini · OAG"/>
          </div>
        </div>
        <div>
          <div className="t-label">LOCATION & SKY</div>
          <div style={{ marginTop: 12 }}>
            <Stat3 k="WHERE"   v="Lyon, France"/>
            <Stat3 k="BORTLE"  v="6 / 9"/>
            <Stat3 k="SQM"     v="19.8 mag/arcsec²"/>
            <Stat3 k="MEMBER"  v="Mar 2024 · 14 mo"/>
          </div>
        </div>
      </section>

      {/* === Stats row === */}
      <section style={{
        padding: "32px 64px",
        borderBottom: "1px solid var(--border-subtle)",
        display: "flex", gap: 48,
      }}>
        <Big n="47" l="published frames"/>
        <Big n="318h" l="total integration"/>
        <Big n="2,841" l="followers"/>
        <Big n="14,206" l="appreciations" accent/>
        <Big n="23" l="targets shot"/>
      </section>

      {/* === Featured row === */}
      <section style={{ padding: "48px 64px 0" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "end" }}>
          <div>
            <Eyebrow2 accent>● FEATURED · PINNED BY MARIE</Eyebrow2>
            <Display2 size={32} style={{ marginTop: 8 }}>Six <em>frames</em> she stands behind</Display2>
          </div>
          <div className="t-meta" style={{ color: "var(--fg-muted)" }}>UPDATED 2 DAYS AGO</div>
        </div>
        <div style={{
          marginTop: 24,
          display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 12,
        }}>
          {featured.map((p, i) => (
            <div key={i} style={{ position: "relative", aspectRatio: "3/4" }}>
              <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
              <div style={{
                position: "absolute", left: 8, top: 8,
                width: 22, height: 22, borderRadius: 0,
                background: "var(--accent)", color: "var(--accent-ink)",
                fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600,
                display: "flex", alignItems: "center", justifyContent: "center",
              }}>{i + 1}</div>
              <div style={{
                position: "absolute", left: 0, right: 0, bottom: 0,
                padding: 12,
                background: "linear-gradient(to top, rgba(0,0,0,.85), transparent)",
              }}>
                <div style={{
                  fontFamily: "var(--font-display)", fontSize: 14, fontStyle: "italic",
                  color: "var(--fg-primary)",
                }}>{p.target}</div>
                <div className="t-meta" style={{ marginTop: 2, color: "var(--fg-muted)" }}>
                  {p.integration} · ♡ {[412, 348, 248, 187, 156, 134][i]}
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* === Justified-rows gallery === */}
      <section style={{ padding: "56px 64px 0" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "end" }}>
          <div>
            <Eyebrow2>FRAMES · ALL 47 PUBLISHED</Eyebrow2>
            <Display2 size={32} style={{ marginTop: 8 }}>The full <em>archive</em></Display2>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="chip chip-accent">Sort: newest ▾</button>
            <button className="chip">Filter: any target ▾</button>
            <button className="chip">Filter: any equipment ▾</button>
          </div>
        </div>

        <div style={{ marginTop: 24 }}>
          {/* row 1 — three landscapes + one square */}
          <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
            <Tile photo={gallery[0]} flex={1.5} h={220} likes={412}/>
            <Tile photo={gallery[1]} flex={1.0} h={220} likes={134}/>
            <Tile photo={gallery[2]} flex={1.4} h={220} likes={248}/>
            <Tile photo={gallery[3]} flex={1.0} h={220} likes={87}/>
          </div>
          {/* row 2 */}
          <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
            <Tile photo={gallery[4]} flex={1.7} h={220} likes={156}/>
            <Tile photo={gallery[5]} flex={1.3} h={220} likes={203}/>
            <Tile photo={gallery[6]} flex={1.0} h={220} likes={62}/>
            <Tile photo={gallery[7]} flex={1.5} h={220} likes={311}/>
          </div>
          {/* row 3 — wide panorama */}
          <div style={{ display: "flex", gap: 8 }}>
            <Tile photo={gallery[8]} flex={2.5} h={220} likes={198}/>
            <Tile photo={gallery[9]} flex={1.0} h={220} likes={78}/>
            <Tile photo={gallery[10]} flex={1.4} h={220} likes={156}/>
          </div>
        </div>

        <div style={{ marginTop: 32, textAlign: "center" }}>
          <button className="btn btn-secondary btn-lg">Load 23 more frames →</button>
        </div>
      </section>
    </div>
  );
};

function EqRow({ k, v }) {
  return (
    <div style={{
      display: "flex", justifyContent: "space-between",
      padding: "8px 0", borderBottom: "1px dashed var(--border-subtle)",
    }}>
      <span className="t-meta" style={{ color: "var(--fg-muted)" }}>{k}</span>
      <span style={{ color: "var(--fg-primary)" }}>{v}</span>
    </div>
  );
}
function Stat3({ k, v }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <div className="t-meta" style={{ color: "var(--fg-muted)" }}>{k}</div>
      <div style={{ fontFamily: "var(--font-display)", fontSize: 18, fontStyle: "italic", color: "var(--fg-primary)", marginTop: 2 }}>{v}</div>
    </div>
  );
}
function Big({ n, l, accent }) {
  return (
    <div>
      <div style={{
        fontFamily: "var(--font-display)", fontSize: 40, lineHeight: 1,
        color: accent ? "var(--accent)" : "var(--fg-primary)",
      }}>{n}</div>
      <div className="t-meta" style={{ marginTop: 6 }}>{l.toUpperCase()}</div>
    </div>
  );
}
function Tile({ photo, flex, h, likes }) {
  const Photo = window.Photo;
  return (
    <div style={{ position: "relative", flex, height: h }}>
      <Photo photo={photo} style={{ position: "absolute", inset: 0 }}/>
      <div style={{
        position: "absolute", inset: 0,
        background: "linear-gradient(to top, rgba(0,0,0,.7) 0%, transparent 50%)",
        opacity: 0, transition: "opacity .2s",
      }}/>
      <div style={{
        position: "absolute", left: 8, bottom: 8, right: 8,
        display: "flex", justifyContent: "space-between", alignItems: "end",
      }}>
        <div className="chip" style={{
          background: "rgba(12,10,8,.7)", borderColor: "var(--border-default)",
          fontSize: 11,
        }}>{photo.target.split("·")[0].trim()}</div>
        <div className="chip" style={{
          background: "rgba(12,10,8,.7)", borderColor: "var(--border-default)",
          color: "var(--accent)",
        }}>♡ {likes}</div>
      </div>
    </div>
  );
}
window.AP_Tile = Tile;

/* ============================================================
   2B — HERO PAGE · OWNER VIEW (empty-state prompts visible)
   ============================================================ */
window.ScreenHeroOwner = function ({ marks }) {
  const AppHeader = window.AppHeader;
  return (
    <div className="screen" style={{ width: 1440, height: 1500, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>

      {/* Owner banner */}
      <div style={{
        padding: "12px 64px", background: "var(--bg-accent-tint)",
        borderBottom: "1px solid var(--accent-dim)",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span className="t-eyebrow" style={{ color: "var(--accent)" }}>● VIEWING YOUR OWN PROFILE · OWNER MODE</span>
          <span className="t-meta" style={{ color: "var(--fg-muted)" }}>VISITORS WON'T SEE THE PROMPTS BELOW</span>
        </div>
        <button className="btn btn-secondary btn-sm">Edit profile</button>
      </div>

      {/* Empty cover */}
      <div style={{
        position: "relative", height: 320,
        background: "var(--bg-base)",
        backgroundImage: `repeating-linear-gradient(45deg, var(--bg-base) 0 12px, var(--bg-raised) 12px 13px)`,
        display: "flex", alignItems: "center", justifyContent: "center",
        borderBottom: "1px solid var(--border-subtle)",
      }}>
        <div style={{ textAlign: "center" }}>
          <Eyebrow2 accent>● COVER EMPTY</Eyebrow2>
          <div style={{
            marginTop: 12, fontFamily: "var(--font-display)", fontSize: 28,
            fontStyle: "italic", color: "var(--accent)",
          }}>Pick a cover from your gallery →</div>
          <div style={{ marginTop: 16, display: "flex", gap: 8, justifyContent: "center" }}>
            <button className="btn btn-primary">Open cover picker</button>
            <button className="btn btn-ghost">Skip for now</button>
          </div>
        </div>
      </div>

      {/* Identity (with prompts) */}
      <section style={{
        padding: "48px 64px 0",
        display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 32, alignItems: "start",
      }}>
        <div style={{
          width: 120, height: 120,
          border: "1px dashed var(--accent)", background: "var(--bg-accent-tint)",
          display: "flex", alignItems: "center", justifyContent: "center",
          fontFamily: "var(--font-display)", fontSize: 48, color: "var(--accent)",
        }}>M</div>
        <div>
          <Display2 size={44}>Marie Dubois</Display2>
          <PromptRow text="Add a tagline" cta="+ Add"/>
          <PromptRow text="Tell visitors about your astrophotography" cta="+ Add bio"/>
        </div>
        <button className="btn btn-secondary">Edit profile</button>
      </section>

      {/* Empty equipment / location prompts */}
      <section style={{
        padding: "32px 64px", marginTop: 32,
        borderTop: "1px solid var(--border-subtle)",
        display: "grid", gridTemplateColumns: "1fr 1fr", gap: 48,
      }}>
        <PromptCard title="EQUIPMENT" cta="Add the gear behind your shots" buttonLabel="+ Add equipment"/>
        <PromptCard title="LOCATION & SKY" cta="Where do you observe from?" buttonLabel="+ Add location"/>
      </section>

      {/* Featured slot prompt */}
      <section style={{ padding: "48px 64px" }}>
        <Eyebrow2 accent>● FEATURED — EMPTY · PIN 3–6 OF YOUR BEST</Eyebrow2>
        <div style={{ marginTop: 24, display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 12 }}>
          {[0, 1, 2, 3, 4, 5].map(i => (
            <div key={i} style={{
              aspectRatio: "3/4", border: "1px dashed var(--border-default)",
              display: "flex", alignItems: "center", justifyContent: "center",
              background: i === 0 ? "var(--bg-accent-tint)" : "var(--bg-base)",
              borderColor: i === 0 ? "var(--accent)" : undefined,
              flexDirection: "column", gap: 8,
            }}>
              <div style={{
                fontFamily: "var(--font-mono)", fontSize: 11,
                color: i === 0 ? "var(--accent)" : "var(--fg-faint)",
                letterSpacing: "0.1em",
              }}>SLOT {String(i + 1).padStart(2, "0")}</div>
              {i === 0 && <div style={{
                fontFamily: "var(--font-display)", fontSize: 13, fontStyle: "italic",
                color: "var(--accent)",
              }}>+ Pin a photo</div>}
            </div>
          ))}
        </div>
        <div className="t-meta" style={{ marginTop: 16, color: "var(--fg-muted)" }}>
          DRAG-AND-DROP TO REORDER · POSITIONS 1–6 PERSIST PER USER
        </div>
      </section>
    </div>
  );
};

function PromptRow({ text, cta }) {
  return (
    <div style={{
      marginTop: 14, padding: "8px 12px",
      border: "1px dashed var(--accent-dim)", background: "var(--bg-accent-tint)",
      display: "flex", justifyContent: "space-between", alignItems: "center",
      maxWidth: 420,
    }}>
      <span style={{ fontFamily: "var(--font-display)", fontSize: 15, fontStyle: "italic", color: "var(--accent)" }}>{text}</span>
      <span className="t-meta" style={{ color: "var(--accent)" }}>{cta}</span>
    </div>
  );
}
function PromptCard({ title, cta, buttonLabel }) {
  return (
    <div style={{
      padding: 24, border: "1px dashed var(--accent-dim)",
      background: "var(--bg-accent-tint)",
    }}>
      <div className="t-label" style={{ color: "var(--accent)" }}>● {title} EMPTY</div>
      <div style={{
        marginTop: 12, fontFamily: "var(--font-display)", fontSize: 22, fontStyle: "italic",
        color: "var(--fg-primary)",
      }}>{cta}</div>
      <button className="btn btn-secondary btn-sm" style={{ marginTop: 14 }}>{buttonLabel}</button>
    </div>
  );
}

/* ============================================================
   2C — PROFILE EDITOR (drawer)
   ============================================================ */
window.ScreenProfileEditor = function ({ marks }) {
  return (
    <div className="screen" style={{
      width: 720, height: 1900, padding: 0,
      background: "var(--bg-raised)",
      borderLeft: "1px solid var(--border-default)", overflow: "hidden",
    }}>
      <div style={{
        padding: "20px 32px",
        borderBottom: "1px solid var(--border-subtle)",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <div>
          <Eyebrow2>EDIT YOUR PROFILE · SAVES ON BLUR</Eyebrow2>
          <Display2 size={28} style={{ marginTop: 4 }}>Profile <em>editor</em></Display2>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-ghost btn-sm">Discard</button>
          <button className="btn btn-primary btn-sm">Save & close</button>
        </div>
      </div>

      <div style={{ padding: "24px 32px" }}>

        <EditorSection num="01" title="IDENTITY">
          <div style={{ display: "flex", gap: 16, alignItems: "center" }}>
            <div style={{
              width: 72, height: 72, background: "var(--accent)",
              color: "var(--accent-ink)", display: "flex",
              alignItems: "center", justifyContent: "center",
              fontFamily: "var(--font-display)", fontSize: 32,
            }}>M</div>
            <div>
              <button className="btn btn-secondary btn-sm">Upload new avatar</button>
              <div className="t-meta" style={{ marginTop: 6, color: "var(--fg-muted)" }}>JPG / PNG · UP TO 4 MB · SQUARE CROP</div>
            </div>
          </div>
          <div style={{ marginTop: 16 }}>
            <div className="t-label" style={{ marginBottom: 6 }}>DISPLAY NAME</div>
            <input className="input" defaultValue="Marie Dubois"/>
          </div>
          <div style={{ marginTop: 16 }}>
            <div className="t-label" style={{ marginBottom: 6 }}>TAGLINE · ONE LINE</div>
            <input className="input" defaultValue="Hunting deep-sky from a Bortle 6 backyard."/>
          </div>
          <div style={{ marginTop: 16 }}>
            <div className="t-label" style={{ marginBottom: 6 }}>HANDLE · @MARIE-DUBOIS</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input className="input input-mono" defaultValue="marie-dubois"/>
              <button className="btn btn-secondary btn-sm">Change</button>
            </div>
            <div className="t-meta" style={{ marginTop: 6, color: "var(--fg-muted)" }}>
              OLD HANDLES REDIRECT FOR 90 DAYS · LAST CHANGED · NEVER
            </div>
          </div>
        </EditorSection>

        <EditorSection num="02" title="ABOUT · RICH TEXT">
          <div style={{ border: "1px solid var(--border-default)" }}>
            <div style={{
              display: "flex", gap: 4, padding: 8,
              borderBottom: "1px solid var(--border-subtle)", background: "var(--bg-base)",
              fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--fg-secondary)",
            }}>
              {["B", "I", "U", "H₂", "H₃", "•", "1.", "❝", "‹›", "🔗"].map(t => (
                <span key={t} style={{
                  padding: "4px 8px", border: "1px solid var(--border-subtle)",
                  background: "var(--bg-raised)",
                }}>{t}</span>
              ))}
            </div>
            <div style={{ padding: 14, fontSize: 14, color: "var(--fg-secondary)", lineHeight: 1.7, minHeight: 180 }}>
              <p style={{ margin: "0 0 12px" }}>I shoot from a small balcony in central Lyon, Bortle 6 with the city's light-dome eating the southern horizon. Mostly narrowband through the FSQ-106 — Hα and OIII are still the only signal I can pull cleanly through the orange.</p>
              <p style={{ margin: 0 }}>I publish only what survives at least <em>six clear hours</em> of integration. Anything less goes into the drafts pile and waits for next season.</p>
            </div>
          </div>
          <div className="t-meta" style={{ marginTop: 8, color: "var(--fg-muted)" }}>
            SANITIZED SERVER-SIDE · SCRIPT, ON*, JAVASCRIPT: STRIPPED · LINKS GET REL=NOFOLLOW
          </div>
        </EditorSection>

        <EditorSection num="03" title="EQUIPMENT · DEFAULT LOADOUT">
          <p style={{ margin: "0 0 12px", color: "var(--fg-muted)", fontSize: 13 }}>
            Pre-fills the upload form. Each entry feeds the equipment dictionary that powers <code className="t-mono" style={{ color: "var(--accent)" }}>/equip/&lt;kind&gt;/&lt;slug&gt;</code>.
          </p>
          {[
            ["SCOPE",    "Takahashi FSQ-106EDX4"],
            ["CAMERA",   "ZWO ASI2600MC Pro"],
            ["MOUNT",    "10Micron GM1000 HPS"],
            ["FILTERS",  "Antlia 3 nm SHO"],
            ["GUIDING",  "ASI120MM Mini · OAG"],
          ].map(([k, v]) => (
            <div key={k} style={{ display: "grid", gridTemplateColumns: "100px 1fr 90px", gap: 12, marginBottom: 8, alignItems: "center" }}>
              <span className="t-label">{k}</span>
              <input className="input input-mono" defaultValue={v}/>
              <span className="t-meta" style={{ color: "var(--accent)" }}>↘ AUTOCOMPLETE</span>
            </div>
          ))}
        </EditorSection>

        <EditorSection num="04" title="LOCATION & SKY">
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <div className="t-label" style={{ marginBottom: 6 }}>CITY / REGION</div>
              <input className="input" defaultValue="Lyon, France"/>
              <div className="t-meta" style={{ marginTop: 6, color: "var(--fg-muted)" }}>NEVER COORDINATES — ONLY CITY-LEVEL</div>
            </div>
            <div>
              <div className="t-label" style={{ marginBottom: 6 }}>SQM (OPTIONAL)</div>
              <input className="input input-mono" defaultValue="19.80"/>
            </div>
            <div style={{ gridColumn: "1 / -1" }}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                <span className="t-label">BORTLE CLASS</span>
                <span className="t-meta" style={{ color: "var(--accent)" }}>● 6 / 9 · BRIGHT SUBURBAN</span>
              </div>
              <div style={{ display: "flex", gap: 4 }}>
                {[1, 2, 3, 4, 5, 6, 7, 8, 9].map(b => (
                  <div key={b} style={{
                    flex: 1, height: 32,
                    background: b === 6 ? "var(--accent)" : "var(--bg-base)",
                    border: "1px solid " + (b === 6 ? "var(--accent)" : "var(--border-default)"),
                    color: b === 6 ? "var(--accent-ink)" : "var(--fg-muted)",
                    fontFamily: "var(--font-mono)", fontSize: 12,
                    display: "flex", alignItems: "center", justifyContent: "center",
                  }}>{b}</div>
                ))}
              </div>
            </div>
          </div>
        </EditorSection>

        <EditorSection num="05" title="SOCIAL LINKS">
          {[
            ["TWITTER",   "https://twitter.com/marie_astro"],
            ["INSTAGRAM", "https://instagram.com/marie.dubois.astro"],
            ["ASTROBIN",  "https://astrobin.com/users/marie-dubois"],
            ["WEBSITE",   "https://marie.photo"],
          ].map(([k, v]) => (
            <div key={k} style={{ display: "grid", gridTemplateColumns: "100px 1fr 32px", gap: 12, marginBottom: 8, alignItems: "center" }}>
              <span className="t-label">{k}</span>
              <input className="input input-mono" defaultValue={v}/>
              <span style={{ color: "var(--fg-faint)", textAlign: "center" }}>✕</span>
            </div>
          ))}
          <button className="btn btn-secondary btn-sm" style={{ marginTop: 8 }}>+ Add link</button>
        </EditorSection>
      </div>
    </div>
  );
};

function EditorSection({ num, title, children }) {
  return (
    <div style={{ marginBottom: 32, paddingBottom: 32, borderBottom: "1px dashed var(--border-default)" }}>
      <div className="t-eyebrow" style={{ marginBottom: 16 }}>● {num} · {title}</div>
      {children}
    </div>
  );
}

/* ============================================================
   2D — LIGHTBOX (overlay route /u/<handle>/p/<short-id>)
   ============================================================ */
window.ScreenLightbox = function ({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: 1440, height: 900, position: "relative", overflow: "hidden" }}>
      {/* Faded gallery underneath */}
      <AppHeader auth marks={marks}/>
      <div style={{
        position: "absolute", inset: "64px 0 0", padding: "32px 64px",
        opacity: 0.18, filter: "blur(2px)",
      }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12 }}>
          {[0, 1, 2, 3, 4, 5, 6, 7].map(i => (
            <div key={i} style={{ position: "relative", aspectRatio: "4/3" }}>
              <Photo photo={PHOTOS_H[i]} style={{ position: "absolute", inset: 0 }}/>
            </div>
          ))}
        </div>
      </div>

      {/* Backdrop */}
      <div style={{
        position: "absolute", inset: 0, background: "rgba(6,5,8,.92)",
      }}/>

      {/* Lightbox */}
      <div style={{
        position: "absolute", inset: "32px 32px 32px 32px",
        display: "grid", gridTemplateColumns: "1fr 380px",
        background: "var(--bg-canvas)", border: "1px solid var(--border-default)",
      }}>
        {/* Image */}
        <div style={{ position: "relative", background: "#000" }}>
          <Photo photo={PHOTOS_H[7]} style={{ position: "absolute", inset: 0 }}/>
          {/* prev/next */}
          <button style={navArrow("left")}>←</button>
          <button style={navArrow("right")}>→</button>
          {/* corner */}
          <div style={{
            position: "absolute", top: 16, left: 16,
            padding: "4px 8px", background: "rgba(12,10,8,.85)",
            border: "1px solid var(--border-default)",
            fontFamily: "var(--font-mono)", fontSize: 11,
            color: "var(--accent)", letterSpacing: "0.08em",
          }}>● 14 / 47 · @MARIE-DUBOIS</div>
        </div>

        {/* Side panel */}
        <div style={{ padding: "24px 28px", overflow: "hidden", display: "flex", flexDirection: "column" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start" }}>
            <div>
              <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● NGC 7000 · NORTH AMERICA</div>
              <Display2 size={26} style={{ marginTop: 8 }}>The North <em>America</em></Display2>
            </div>
            <button className="btn btn-ghost" style={{ fontSize: 18, marginTop: -8 }}>✕</button>
          </div>

          <p style={{
            marginTop: 16, color: "var(--fg-secondary)", fontSize: 13, lineHeight: 1.7,
          }}>
            Eighteen hours of SHO over four nights in March. The Pelican (IC 5070) sits to the south-east — visible bottom-left.
          </p>

          {/* Like + comment line */}
          <div style={{
            marginTop: 16, display: "flex", gap: 12, alignItems: "center",
            paddingBottom: 16, borderBottom: "1px dashed var(--border-default)",
          }}>
            <button className="btn btn-secondary btn-sm" style={{
              borderColor: "var(--accent)", color: "var(--accent)",
              background: "var(--bg-accent-tint)",
            }}>♥ 248</button>
            <button className="btn btn-ghost btn-sm">⤴ Share</button>
            <button className="btn btn-ghost btn-sm">↗ Open</button>
            <span style={{ marginLeft: "auto" }} className="t-meta">L · LIKE  ·  I · INFO  ·  ESC</span>
          </div>

          {/* EXIF + equipment */}
          <table className="exif" style={{ marginTop: 16 }}>
            <tbody>
              <tr><th>SCOPE</th><td>Takahashi FSQ-106EDX4</td></tr>
              <tr><th>CAMERA</th><td>ZWO ASI2600MC Pro</td></tr>
              <tr><th>MOUNT</th><td>10Micron GM1000 HPS</td></tr>
              <tr><th>FILTERS</th><td>Antlia 3 nm SHO</td></tr>
              <tr><th>EXPOSURE</th><td>180 × 360 s</td></tr>
              <tr><th>INTEGRATION</th><td>18 h 00 m</td></tr>
              <tr><th>CAPTURED</th><td>14 – 17 Mar 2026</td></tr>
              <tr><th>BORTLE</th><td>6 · Lyon, France</td></tr>
            </tbody>
          </table>

          {/* More from Marie */}
          <div style={{ marginTop: "auto", paddingTop: 20, borderTop: "1px dashed var(--border-default)" }}>
            <div className="t-label" style={{ marginBottom: 8 }}>MORE FROM @MARIE-DUBOIS</div>
            <div style={{ display: "flex", gap: 6 }}>
              {[1, 9, 10, 4].map(i => (
                <div key={i} style={{ position: "relative", flex: 1, aspectRatio: "1" }}>
                  <Photo photo={PHOTOS_H[i]} style={{ position: "absolute", inset: 0 }}/>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

function navArrow(side) {
  return {
    position: "absolute", top: "50%", [side]: 16,
    transform: "translateY(-50%)",
    width: 44, height: 44,
    background: "rgba(12,10,8,.7)",
    border: "1px solid var(--border-default)",
    color: "var(--fg-primary)",
    fontFamily: "var(--font-mono)", fontSize: 18,
    display: "flex", alignItems: "center", justifyContent: "center",
    cursor: "pointer",
  };
}
