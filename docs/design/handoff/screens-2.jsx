/* ===== ASTROPHOTO SCREENS — set 2 =====
   Logged-in home, Upload, My photos, Sign in/up, Settings, 404
*/

const PHOTOS2 = window.PHOTOS;

/* ============================================================
   5. LOGGED-IN HOME / FOLLOWING FEED
   ============================================================ */
window.ScreenHome = function ScreenHome({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1500px", overflow: "hidden" }}>
      <AppHeader active="Gallery" auth marks={marks}/>
      {/* Top bar */}
      <section style={{
        padding: "40px 64px 24px",
        display: "grid", gridTemplateColumns: "1fr auto", gap: 32, alignItems: "end",
        borderBottom: "1px solid var(--border-subtle)",
      }}>
        <div>
          <div className="t-eyebrow" style={{ marginBottom: 8 }}>WELCOME BACK · 14 MARCH 2026 · NEW MOON IN 6 DAYS</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 56, fontWeight: 400, margin: 0, lineHeight: 1 }}>
            Good evening, <em>Marie</em>.
          </h1>
          <p style={{ marginTop: 16, color: "var(--fg-secondary)", fontSize: 15 }}>
            12 new frames from the people you follow · clear skies tonight in Provence.
          </p>
        </div>
        <button className="btn btn-primary btn-lg" style={{ height: 52, padding: "0 28px" }}>
          <span style={{ fontSize: 18 }}>+</span> &nbsp;Upload a frame
        </button>
      </section>

      {/* Tabs */}
      <section style={{ padding: "0 64px", borderBottom: "1px solid var(--border-subtle)", display: "flex", gap: 40 }}>
        {["Following · 12", "All public", "Targets I watch"].map((t,i)=>(
          <a key={t} className={"nav-link" + (i===0?" active":"")} style={{ padding: "16px 0" }}>{t}</a>
        ))}
      </section>

      {/* Asymmetric editorial grid: hero + 2-up */}
      <section style={{ padding: "32px 64px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr", gap: 20, marginBottom: 32 }}>
          <div>
            <div style={{ position: "relative", aspectRatio: "16/10" }}>
              <Photo photo={PHOTOS2[2]} style={{ position: "absolute", inset: 0 }}/>
            </div>
            <div style={{ padding: "12px 4px" }}>
              <div style={{ fontFamily: "var(--font-display)", fontSize: 22, fontStyle: "italic" }}>{PHOTOS2[2].target}</div>
              <div className="t-meta" style={{ marginTop: 4 }}>{PHOTOS2[2].photographer.toUpperCase()} · {PHOTOS2[2].integration} INTEGRATION · 2 H AGO</div>
            </div>
          </div>
          <div style={{ display: "grid", gridTemplateRows: "1fr 1fr", gap: 20 }}>
            {[3, 9].map(idx => (
              <div key={idx}>
                <div style={{ position: "relative", height: "100%" }}>
                  <Photo photo={PHOTOS2[idx]} style={{ position: "absolute", inset: 0 }}/>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Activity log strip */}
        <div style={{ padding: "20px 0", borderTop: "1px solid var(--border-subtle)", borderBottom: "1px solid var(--border-subtle)", marginBottom: 32 }}>
          <div className="t-eyebrow" style={{ marginBottom: 12 }}>RECENT ACTIVITY</div>
          {[
            ["L. Petrov", "appreciated your frame", "M27 · Dumbbell Nebula", "3 h"],
            ["StarHunter42", "started following you", "", "5 h"],
            ["P. Halverson", "commented on", "M51 · Whirlpool", "yesterday"],
          ].map(([who, action, target, time], i) => (
            <div key={i} style={{ display: "flex", padding: "8px 0", fontFamily: "var(--font-mono)", fontSize: 12, borderBottom: i < 2 ? "1px dashed var(--border-subtle)" : "none" }}>
              <span style={{ color: "var(--accent)", width: 160 }}>{who}</span>
              <span style={{ color: "var(--fg-muted)", width: 200 }}>{action}</span>
              <span style={{ color: "var(--fg-primary)", flex: 1 }}>{target}</span>
              <span style={{ color: "var(--fg-faint)" }}>{time}</span>
            </div>
          ))}
        </div>

        {/* More from following */}
        <div className="t-eyebrow" style={{ marginBottom: 16 }}>MORE FROM PEOPLE YOU FOLLOW</div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 16 }}>
          {PHOTOS2.slice(4, 12).map((p,i)=>(
            <div key={i}>
              <div style={{ position: "relative", aspectRatio: "4/3" }}>
                <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
              </div>
              <div style={{ padding: "8px 2px", fontFamily: "var(--font-mono)", fontSize: 11 }}>
                <div style={{ color: "var(--fg-primary)" }}>{p.target}</div>
                <div style={{ color: "var(--fg-muted)" }}>{p.photographer} · {p.integration}</div>
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
};

/* ============================================================
   6. UPLOAD FLOW — three-step with EXIF confirmation
   ============================================================ */
window.ScreenUpload = function ScreenUpload({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1300px", overflow: "hidden" }}>
      <AppHeader active="Gallery" auth marks={marks}/>
      <section style={{ padding: "40px 64px 24px", borderBottom: "1px solid var(--border-subtle)" }}>
        <div className="t-eyebrow">NEW FRAME</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 0", lineHeight: 1 }}>
          Add a <em>frame</em> to your archive
        </h1>
        {/* Stepper */}
        <div style={{ display: "flex", gap: 0, marginTop: 32, fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.12em", textTransform: "uppercase" }}>
          {[["01", "UPLOAD", "done"], ["02", "VERIFY DATA", "active"], ["03", "CAPTION & PUBLISH", ""]].map(([n, l, s]) => (
            <div key={n} style={{
              flex: 1, padding: "16px 0", borderTop: `2px solid ${s ? "var(--accent)" : "var(--border-default)"}`,
              color: s ? "var(--fg-primary)" : "var(--fg-muted)",
              display: "flex", gap: 12, alignItems: "center",
            }}>
              <span style={{ color: s ? "var(--accent)" : "var(--fg-faint)" }}>{n}</span>
              <span>{l}</span>
              {s === "done" && <span style={{color:"var(--accent)", marginLeft:"auto", marginRight: 32}}>✓</span>}
            </div>
          ))}
        </div>
      </section>

      <section style={{ padding: "48px 64px", display: "grid", gridTemplateColumns: "560px 1fr", gap: 64 }}>
        {/* Left: image preview */}
        <div>
          <div className="t-label" style={{ marginBottom: 12 }}>YOUR UPLOAD</div>
          <div style={{ position: "relative", aspectRatio: "4/3" }}>
            <Photo photo={PHOTOS2[7]} style={{ position: "absolute", inset: 0 }}/>
            {/* loading bar */}
            <div style={{ position: "absolute", left: 12, right: 12, bottom: 12, padding: "8px 12px", background: "rgba(12,10,8,.85)", border: "1px solid var(--border-default)", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--fg-secondary)" }}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <span style={{ color: "var(--accent)" }}>● PROCESSING THUMBNAILS</span>
                <span>72%</span>
              </div>
              <div style={{ marginTop: 6, height: 2, background: "var(--border-default)", position: "relative" }}>
                <div style={{ position: "absolute", inset: 0, width: "72%", background: "var(--accent)" }}/>
              </div>
            </div>
          </div>
          <div className="t-meta" style={{ marginTop: 12, display: "flex", justifyContent: "space-between" }}>
            <span>NGC7000_SHO_final.jpg</span>
            <span>32.4 MB · 6248 × 4176</span>
          </div>
        </div>

        {/* Right: EXIF confirm form */}
        <div>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
            <div className="t-label">DETECTED FROM YOUR FILE</div>
            <span className="t-meta" style={{ color: "var(--accent)" }}>● 11 fields recovered from EXIF</span>
          </div>
          <p style={{ color: "var(--fg-secondary)", fontSize: 13, marginTop: 0, marginBottom: 24 }}>
            We've read what your file knew. Correct anything that's wrong — every field is editable, none are required.
          </p>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
            <Field label="Target" mono detected="auto" value="NGC 7000 — North America Nebula" full/>
            <Field label="Captured" mono value="14–17 March 2026"/>
            <Field label="Sessions" mono value="4"/>
            <Field label="Camera" mono detected value="ZWO ASI2600MC Pro"/>
            <Field label="Telescope" mono detected value="Takahashi FSQ-106EDX4"/>
            <Field label="Focal length" mono detected value="530 mm"/>
            <Field label="Aperture" mono detected value="f/5"/>
            <Field label="Mount" mono value="10Micron GM1000 HPS"/>
            <Field label="Filters" mono value="Antlia 3 nm SHO"/>
            <Field label="Exposure" mono detected value="180 × 360 s"/>
            <Field label="Gain" mono detected value="100"/>
            <Field label="Sensor temp" mono value="−10 °C"/>
            <Field label="RA · Dec" mono value="20ʰ 58ᵐ 47ˢ / +44° 19′ 53″" full/>
          </div>

          <div style={{ marginTop: 32, padding: 16, border: "1px solid var(--border-default)", background: "var(--bg-base)" }}>
            <div className="t-label" style={{ color: "var(--accent)", marginBottom: 8 }}>OPTIONAL — PLATE SOLVE</div>
            <p style={{ margin: 0, fontSize: 12, color: "var(--fg-secondary)" }}>
              Run plate-solving to recover RA/Dec, scale, and rotation precisely. Takes ~30 s. <a style={{color:"var(--accent)"}}>Run now →</a>
            </p>
          </div>

          <div style={{ marginTop: 32, display: "flex", gap: 12, justifyContent: "flex-end" }}>
            <button className="btn btn-ghost btn-lg">Save as draft</button>
            <button className="btn btn-secondary btn-lg">← Replace file</button>
            <button className="btn btn-primary btn-lg">Continue to caption →</button>
          </div>
        </div>
      </section>
    </div>
  );
};

function Field({ label, value, mono, detected, full }) {
  return (
    <div style={{ gridColumn: full ? "1 / -1" : "auto" }}>
      <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: 6 }}>
        <span className="t-label">{label}</span>
        {detected && <span className="t-meta" style={{ color: detected === "auto" ? "var(--fg-muted)" : "var(--accent)" }}>{detected === "auto" ? "you fill" : "from EXIF"}</span>}
      </div>
      <input className={"input " + (mono ? "input-mono" : "")} defaultValue={value}/>
    </div>
  );
}

/* ============================================================
   7. MY PHOTOS DASHBOARD
   ============================================================ */
window.ScreenMyPhotos = function ScreenMyPhotos({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1200px", overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <section style={{ padding: "40px 64px 16px", display: "flex", justifyContent: "space-between", alignItems: "end", borderBottom: "1px solid var(--border-subtle)" }}>
        <div>
          <div className="t-eyebrow">YOUR ARCHIVE</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 0" }}>My <em>frames</em></h1>
        </div>
        <div style={{ display: "flex", gap: 32, fontFamily: "var(--font-mono)", fontSize: 12 }}>
          <Stat n="42" l="published"/>
          <Stat n="3" l="drafts"/>
          <Stat n="318 h" l="total integration"/>
          <Stat n="14,206" l="appreciations"/>
        </div>
      </section>

      <section style={{ padding: "20px 64px", borderBottom: "1px solid var(--border-subtle)", display: "flex", justifyContent: "space-between" }}>
        <div style={{ display: "flex", gap: 8 }}>
          {["All · 45", "Published · 42", "Drafts · 3"].map((t,i)=>(
            <button key={t} className={"chip " + (i===0?"chip-accent":"")}>{t}</button>
          ))}
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <button className="chip">Sort: newest ▾</button>
          <button className="chip">Filter: all targets ▾</button>
        </div>
      </section>

      {/* Table */}
      <section style={{ padding: "0 64px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "80px 1fr 200px 140px 120px 100px 80px", padding: "16px 0", borderBottom: "1px solid var(--border-default)", fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.1em", textTransform: "uppercase", color: "var(--fg-muted)" }}>
            <span></span><span>TARGET</span><span>CAPTURED</span><span>INTEGRATION</span><span>STATUS</span><span>♡</span><span></span>
        </div>
        {PHOTOS2.slice(0, 8).map((p, i) => (
          <div key={i} style={{ display: "grid", gridTemplateColumns: "80px 1fr 200px 140px 120px 100px 80px", padding: "12px 0", borderBottom: "1px dashed var(--border-subtle)", alignItems: "center", fontFamily: "var(--font-mono)", fontSize: 13 }}>
            <div style={{ position: "relative", width: 60, height: 60 }}>
              <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
            </div>
            <div>
              <div style={{ color: "var(--fg-primary)", fontFamily: "var(--font-display)", fontSize: 17, fontStyle: "italic" }}>{p.target}</div>
              <div style={{ color: "var(--fg-muted)", fontSize: 11, marginTop: 2 }}>{p.camera}</div>
            </div>
            <span style={{color:"var(--fg-secondary)"}}>{["14 Mar 2026","09 Mar 2026","02 Mar 2026","27 Feb 2026","18 Feb 2026","04 Feb 2026","21 Jan 2026","08 Jan 2026"][i]}</span>
            <span style={{color:"var(--fg-secondary)"}}>{p.integration}</span>
            <span><span className={i===2?"chip":"chip chip-accent"}>{i===2?"DRAFT":"PUBLISHED"}</span></span>
            <span style={{color:"var(--accent)"}}>{[248, 412, 0, 187, 92, 76, 311, 198][i]}</span>
            <span style={{color:"var(--fg-muted)"}}>⋯</span>
          </div>
        ))}
      </section>
    </div>
  );
};

const Stat = ({ n, l }) => (
  <div style={{ textAlign: "right" }}>
    <div style={{ fontFamily: "var(--font-display)", fontSize: 28, color: "var(--fg-primary)" }}>{n}</div>
    <div className="t-meta" style={{ marginTop: 2 }}>{l.toUpperCase()}</div>
  </div>
);

/* ============================================================
   8. SIGN IN
   ============================================================ */
window.ScreenSignIn = function ScreenSignIn({ marks }) {
  const Wordmark = marks.Wordmark;
  const Mark = marks.MarkReticle;
  return (
    <div className="screen" style={{ width: "1440px", height: "900px", display: "grid", gridTemplateColumns: "1fr 1fr" }}>
      {/* Left — image + quote */}
      <div style={{ position: "relative", overflow: "hidden", background: "#000" }}>
        <window.Photo photo={PHOTOS2[10]} style={{ position: "absolute", inset: 0 }}/>
        <div style={{ position: "absolute", inset: 0, background: "linear-gradient(to right, rgba(12,10,8,.85), rgba(12,10,8,.2))" }}/>
        <div style={{ position: "absolute", left: 64, top: 64, display: "flex", alignItems: "center", gap: 14 }}>
          <Mark size={28} color="var(--accent)"/>
          <Wordmark size={24} italic={true}>Astrophoto</Wordmark>
        </div>
        <div style={{ position: "absolute", left: 64, bottom: 64, maxWidth: 480 }}>
          <div className="t-eyebrow" style={{ color: "var(--accent)", marginBottom: 16 }}>● ρ OPHIUCHI · 5h45m · A. DIMOV</div>
          <p style={{ fontFamily: "var(--font-display)", fontSize: 36, fontStyle: "italic", lineHeight: 1.15, margin: 0 }}>
            "The faintest tendrils of dust only show themselves to the patient."
          </p>
        </div>
        <div style={{ position: "absolute", right: 24, bottom: 24, fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--fg-muted)", letterSpacing: "0.12em" }}>
          16ʰ 25ᵐ / −23° 27′
        </div>
      </div>

      {/* Right — form */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", padding: "64px" }}>
        <div style={{ width: "100%", maxWidth: 380 }}>
          <div className="t-eyebrow" style={{ marginBottom: 16 }}>SIGN IN</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 44, fontWeight: 400, margin: 0, lineHeight: 1.05 }}>
            Welcome back<br/>to <em>your archive</em>.
          </h1>
          <p style={{ marginTop: 16, color: "var(--fg-secondary)", fontSize: 14 }}>
            New here? <a style={{ color: "var(--accent)" }}>Open an account →</a>
          </p>
          <div style={{ marginTop: 40 }}>
            <button className="btn btn-secondary btn-lg" style={{ width: "100%", justifyContent: "center" }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M22 12.5C22 6.7 17.5 2 12 2S2 6.7 2 12.5 6.5 23 12 23s10-4.7 10-10.5z"/></svg>
              Continue with Google
            </button>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 16, margin: "24px 0", color: "var(--fg-faint)" }}>
            <div style={{ flex: 1, height: 1, background: "var(--border-subtle)" }}/>
            <span className="t-meta">OR</span>
            <div style={{ flex: 1, height: 1, background: "var(--border-subtle)" }}/>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div>
              <div className="t-label" style={{ marginBottom: 6 }}>EMAIL</div>
              <input className="input" defaultValue="marie.dubois@example.fr"/>
            </div>
            <div>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                <span className="t-label">PASSWORD</span>
                <a className="t-meta" style={{ color: "var(--accent)" }}>Forgot?</a>
              </div>
              <input className="input" type="password" defaultValue="••••••••••"/>
            </div>
            <button className="btn btn-primary btn-lg" style={{ marginTop: 8 }}>Sign in</button>
          </div>
        </div>
      </div>
    </div>
  );
};

/* ============================================================
   9. SIGN UP
   ============================================================ */
window.ScreenSignUp = function ScreenSignUp({ marks }) {
  const Wordmark = marks.Wordmark;
  return (
    <div className="screen" style={{ width: "720px", height: "900px", padding: "64px", display: "flex", flexDirection: "column", justifyContent: "center" }}>
      <Wordmark size={32} italic={true}>Astrophoto</Wordmark>
      <div className="t-eyebrow" style={{ marginTop: 48, marginBottom: 16 }}>OPEN AN ACCOUNT</div>
      <h1 style={{ fontFamily: "var(--font-display)", fontSize: 44, fontWeight: 400, margin: 0, lineHeight: 1.05 }}>
        A serious home for<br/><em>the work you make</em>.
      </h1>
      <p style={{ marginTop: 16, color: "var(--fg-secondary)", fontSize: 14, maxWidth: 480 }}>
        Free, no ads, no rankings. Your photos with their full technical record, kept for as long as you want them kept.
      </p>
      <div style={{ marginTop: 40, maxWidth: 480 }}>
        <button className="btn btn-secondary btn-lg" style={{ width: "100%", justifyContent: "center" }}>
          Continue with Google
        </button>
        <div style={{ display: "flex", alignItems: "center", gap: 16, margin: "24px 0" }}>
          <div style={{ flex: 1, height: 1, background: "var(--border-subtle)" }}/>
          <span className="t-meta">OR WITH EMAIL</span>
          <div style={{ flex: 1, height: 1, background: "var(--border-subtle)" }}/>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
          <div><div className="t-label" style={{marginBottom:6}}>DISPLAY NAME</div><input className="input" placeholder="How others will see you"/></div>
          <div><div className="t-label" style={{marginBottom:6}}>EMAIL</div><input className="input" placeholder="you@somewhere.com"/></div>
          <div><div className="t-label" style={{marginBottom:6}}>PASSWORD</div><input className="input" type="password" placeholder="At least 10 characters"/></div>
          <button className="btn btn-primary btn-lg" style={{ marginTop: 8 }}>Create my account</button>
          <p className="t-meta" style={{ marginTop: 4, lineHeight: 1.6 }}>
            By continuing you agree to our <a style={{color:"var(--accent)"}}>terms</a> and <a style={{color:"var(--accent)"}}>privacy policy</a>.
            We don't ask for, and never sell, your data.
          </p>
        </div>
      </div>
    </div>
  );
};

/* ============================================================
   10. ACCOUNT SETTINGS
   ============================================================ */
window.ScreenSettings = function ScreenSettings({ marks }) {
  const AppHeader = window.AppHeader;
  return (
    <div className="screen" style={{ width: "1440px", height: "1100px", overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <section style={{ padding: "40px 64px 0" }}>
        <div className="t-eyebrow">PREFERENCES</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 0" }}>Account <em>settings</em></h1>
      </section>
      <section style={{ display: "grid", gridTemplateColumns: "240px 1fr", gap: 64, padding: "40px 64px" }}>
        <nav style={{ display: "flex", flexDirection: "column", gap: 4, fontFamily: "var(--font-mono)", fontSize: 12 }}>
          {[["PROFILE","active"],["EQUIPMENT",""],["NOTIFICATIONS",""],["EMAIL & SECURITY",""],["APPEARANCE",""],["SESSIONS",""],["DELETE ACCOUNT","danger"]].map(([n,s]) => (
            <a key={n} style={{
              padding: "10px 12px",
              letterSpacing: "0.12em",
              color: s==="active" ? "var(--accent)" : s==="danger" ? "var(--danger)" : "var(--fg-muted)",
              borderLeft: s==="active" ? "1px solid var(--accent)" : "1px solid transparent",
              background: s==="active" ? "rgba(232,164,58,.06)" : "transparent",
            }}>{n}</a>
          ))}
        </nav>
        <div style={{ maxWidth: 640 }}>
          <Section title="Identity" desc="How others see you across Astrophoto.">
            <Row label="DISPLAY NAME"><input className="input" defaultValue="Marie Dubois"/></Row>
            <Row label="HANDLE"><input className="input input-mono" defaultValue="@marie.dubois"/></Row>
            <Row label="ABOUT"><textarea className="textarea" defaultValue="Deep-sky narrowband from Bortle 4 in Haute-Provence."/></Row>
            <Row label="LOCATION"><input className="input" defaultValue="Saint-Étienne-les-Orgues, FR"/></Row>
          </Section>
          <Section title="Appearance" desc="Astrophoto defaults to dark. Most practitioners keep it dark.">
            <Row label="THEME">
              <div style={{ display: "flex", gap: 8 }}>
                <button className="chip chip-accent">DARK · DEFAULT</button>
                <button className="chip">LIGHT</button>
                <button className="chip">SYSTEM</button>
              </div>
            </Row>
            <Row label="DENSITY">
              <div style={{ display: "flex", gap: 8 }}>
                <button className="chip chip-accent">SHOW THE WORK</button>
                <button className="chip">SHOW THE DATA</button>
              </div>
            </Row>
          </Section>
          <div style={{ marginTop: 32, display: "flex", gap: 12, justifyContent: "flex-end" }}>
            <button className="btn btn-ghost">Discard</button>
            <button className="btn btn-primary">Save changes</button>
          </div>
        </div>
      </section>
    </div>
  );
};

const Section = ({ title, desc, children }) => (
  <div style={{ paddingBottom: 32, borderBottom: "1px solid var(--border-subtle)", marginBottom: 32 }}>
    <h2 style={{ fontFamily: "var(--font-display)", fontSize: 24, fontWeight: 400, margin: "0 0 4px", fontStyle: "italic" }}>{title}</h2>
    <p style={{ color: "var(--fg-muted)", fontSize: 13, margin: "0 0 24px" }}>{desc}</p>
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>{children}</div>
  </div>
);
const Row = ({ label, children }) => (
  <div style={{ display: "grid", gridTemplateColumns: "140px 1fr", gap: 24, alignItems: "start" }}>
    <span className="t-label" style={{ paddingTop: 10 }}>{label}</span>
    <div>{children}</div>
  </div>
);

/* ============================================================
   11. 404 / EMPTY STATES
   ============================================================ */
window.Screen404 = function Screen404({ marks }) {
  const AppHeader = window.AppHeader;
  const MarkReticle = marks.MarkReticle;
  return (
    <div className="screen" style={{ width: "1440px", height: "900px", overflow: "hidden", display: "flex", flexDirection: "column" }}>
      <AppHeader marks={marks}/>
      <div className="bg-grid" style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", position: "relative" }}>
        <div style={{ textAlign: "center", maxWidth: 540 }}>
          <MarkReticle size={88} color="var(--accent)"/>
          <div className="t-eyebrow" style={{ marginTop: 32, color: "var(--accent)" }}>● 404 · NO LIGHT FROM THIS DIRECTION</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 56, fontWeight: 400, margin: "16px 0 0", lineHeight: 1 }}>
            We pointed the scope at <em>nothing</em>.
          </h1>
          <p style={{ color: "var(--fg-secondary)", fontSize: 15, marginTop: 16 }}>
            The page you asked for is below the horizon — moved, deleted, or it never rose.
            Try the gallery, or check the address.
          </p>
          <div style={{ marginTop: 32, fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--fg-muted)", letterSpacing: "0.08em" }}>
            REQUESTED · /photo/ngc-7000-from-2019<br/>
            COORDINATES · UNRESOLVED
          </div>
          <div style={{ marginTop: 32, display: "flex", gap: 12, justifyContent: "center" }}>
            <button className="btn btn-primary btn-lg">Back to gallery</button>
            <button className="btn btn-secondary btn-lg">Search the archive</button>
          </div>
        </div>
      </div>
    </div>
  );
};

/* ============================================================
   12. EMPTY STATE — new user, no uploads
   ============================================================ */
window.ScreenEmpty = function ScreenEmpty({ marks }) {
  const AppHeader = window.AppHeader;
  const MarkAtlas = marks.MarkAtlas;
  return (
    <div className="screen" style={{ width: "1440px", height: "900px", overflow: "hidden", display: "flex", flexDirection: "column" }}>
      <AppHeader auth marks={marks}/>
      <section style={{ padding: "40px 64px 0" }}>
        <div className="t-eyebrow">YOUR ARCHIVE</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 0" }}>My <em>frames</em></h1>
      </section>
      <div className="bg-grid" style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", position: "relative" }}>
        <div style={{ textAlign: "center", maxWidth: 540, position: "relative", zIndex: 1 }}>
          <MarkAtlas size={104} color="var(--fg-faint)"/>
          <h2 style={{ fontFamily: "var(--font-display)", fontSize: 40, fontWeight: 400, margin: "32px 0 0", lineHeight: 1.05 }}>
            <em>An empty plate</em>,<br/>waiting for first light.
          </h2>
          <p style={{ color: "var(--fg-secondary)", fontSize: 14, marginTop: 16, maxWidth: 420, marginLeft: "auto", marginRight: "auto" }}>
            When you upload your first frame, it will live here — with its full technical record, in your colors, on your terms.
          </p>
          <div style={{ marginTop: 32 }}>
            <button className="btn btn-primary btn-lg">Upload your first frame</button>
          </div>
          <div style={{ marginTop: 48, paddingTop: 32, borderTop: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--fg-muted)", letterSpacing: "0.06em" }}>
            NEW HERE? &nbsp; <a style={{color:"var(--accent)"}}>READ THE SHORT GUIDE →</a>
          </div>
        </div>
      </div>
    </div>
  );
};
