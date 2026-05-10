/* ===== ASTROPHOTO PHASE 8 — My Photos v2, Replace flow, Polish items ===== */

const PHOTOS_P8B = window.PHOTOS;

/* ============================================================
   5. MY PHOTOS v2 — desktop with drafts surface
   ============================================================ */
window.ScreenMyPhotosV2 = function ScreenMyPhotosV2({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  const photos = PHOTOS_P8B;
  const draftIdx = [2, 5, 11];
  return (
    <div className="screen" style={{ width: "1440px", height: "1400px", overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <section style={{ padding: "40px 64px 16px", display: "flex", justifyContent: "space-between", alignItems: "end", borderBottom: "1px solid var(--border-subtle)" }}>
        <div>
          <div className="t-eyebrow">YOUR ARCHIVE</div>
          <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 0" }}>My <em>frames</em></h1>
        </div>
        <div style={{ display: "flex", gap: 32, fontFamily: "var(--font-mono)", fontSize: 12 }}>
          <StatP8 n="42" l="published"/>
          <StatP8 n="3" l="drafts" accent/>
          <StatP8 n="318 h" l="total integration"/>
          <StatP8 n="14,206" l="appreciations"/>
        </div>
      </section>

      {/* Drafts callout — surfaced */}
      <section style={{ padding: "24px 64px", background: "var(--bg-warning-tint)", borderBottom: "1px solid var(--warning)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
          <div>
            <div className="t-eyebrow" style={{ color: "var(--warning)" }}>● 3 DRAFTS · NOT YET PUBLISHED</div>
            <p style={{ margin: "6px 0 0", color: "var(--fg-secondary)", fontSize: 13 }}>
              Frames you started uploading but haven't shared. Drafts are private to you.
            </p>
          </div>
          <a className="t-meta" style={{ color: "var(--accent)" }}>SEE ALL DRAFTS →</a>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16 }}>
          {draftIdx.map((idx, i) => {
            const p = photos[idx];
            return (
              <div key={i} style={{ display: "flex", gap: 12, padding: 12, background: "var(--bg-raised)", border: "1px dashed var(--border-default)" }}>
                <div style={{ position: "relative", width: 80, height: 80, flexShrink: 0 }}>
                  <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
                  <div style={{ position: "absolute", inset: 0, background: "rgba(12,10,8,.4)" }}/>
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontFamily: "var(--font-display)", fontSize: 15, fontStyle: "italic" }}>{p.target}</div>
                  <div className="t-meta" style={{ marginTop: 4 }}>STEP 02 · VERIFYING DATA · 11 DAYS AGO</div>
                  <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
                    <button className="btn btn-primary btn-sm">Continue →</button>
                    <button className="btn btn-ghost btn-sm">Discard</button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </section>

      <section style={{ padding: "20px 64px", borderBottom: "1px solid var(--border-subtle)", display: "flex", justifyContent: "space-between" }}>
        <div style={{ display: "flex", gap: 8 }}>
          {[["All · 45", true], ["Published · 42", false], ["Drafts · 3", false]].map(([t, on], i) => (
            <button key={i} className={"chip " + (on ? "chip-accent" : "")}>{t}</button>
          ))}
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <button className="chip">Sort: newest ▾</button>
          <button className="chip">Filter: all targets ▾</button>
          <button className="chip">View: list ▾</button>
        </div>
      </section>

      <section style={{ padding: "0 64px" }}>
        <div style={{ display: "grid", gridTemplateColumns: "80px 1fr 200px 140px 120px 100px 80px", padding: "16px 0", borderBottom: "1px solid var(--border-default)", fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.1em", textTransform: "uppercase", color: "var(--fg-muted)" }}>
          <span></span><span>TARGET</span><span>CAPTURED</span><span>INTEGRATION</span><span>STATUS</span><span>♡</span><span></span>
        </div>
        {photos.slice(0, 9).map((p, i) => {
          const isDraft = draftIdx.includes(i);
          const isUntitled = i === 6;
          return (
            <div key={i} style={{
              display: "grid", gridTemplateColumns: "80px 1fr 200px 140px 120px 100px 80px",
              padding: "12px 0", borderBottom: "1px dashed var(--border-subtle)",
              alignItems: "center", fontFamily: "var(--font-mono)", fontSize: 13,
              opacity: isDraft ? 0.78 : 1,
            }}>
              <div style={{ position: "relative", width: 60, height: 60 }}>
                <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
                {isDraft && <div style={{ position: "absolute", inset: 0, background: "rgba(12,10,8,.4)", border: "1px dashed var(--warning)" }}/>}
              </div>
              <div>
                <div style={{
                  color: isUntitled ? "var(--fg-muted)" : "var(--fg-primary)",
                  fontFamily: "var(--font-display)", fontSize: 17, fontStyle: "italic",
                }}>
                  {isUntitled ? <span>IC434_HORSE_v3.tif <span className="t-meta" style={{ color: "var(--fg-faint)", fontStyle: "normal" }}>· UNTITLED · FROM FILENAME</span></span> : p.target}
                </div>
                <div style={{ color: "var(--fg-muted)", fontSize: 11, marginTop: 2 }}>{p.camera}</div>
              </div>
              <span style={{color:"var(--fg-secondary)"}}>{["14 Mar","09 Mar","02 Mar","27 Feb","18 Feb","04 Feb","21 Jan","08 Jan","02 Jan"][i]} 2026</span>
              <span style={{color:"var(--fg-secondary)"}}>{p.integration}</span>
              <span><span className={"chip" + (isDraft ? "" : " chip-accent")} style={isDraft ? { borderColor: "var(--warning)", color: "var(--warning)" } : {}}>{isDraft ? "DRAFT" : "PUBLISHED"}</span></span>
              <span style={{color: isDraft ? "var(--fg-faint)" : "var(--accent)"}}>{isDraft ? "—" : [248, 412, 0, 187, 92, 76, 311, 198, 156][i]}</span>
              <span style={{color:"var(--fg-muted)"}}>⋯</span>
            </div>
          );
        })}
      </section>
    </div>
  );
};

const StatP8 = ({ n, l, accent }) => (
  <div style={{ textAlign: "right" }}>
    <div style={{ fontFamily: "var(--font-display)", fontSize: 28, color: accent ? "var(--accent)" : "var(--fg-primary)" }}>{n}</div>
    <div className="t-meta" style={{ marginTop: 2 }}>{l.toUpperCase()}</div>
  </div>
);

/* ============================================================
   6. MY PHOTOS — mobile (390)
   ============================================================ */
window.ScreenMyPhotosMobile = function ScreenMyPhotosMobile({ marks }) {
  const Photo = window.Photo;
  const photos = PHOTOS_P8B;
  const Mark = marks.MarkReticle;
  return (
    <div className="screen" style={{ width: "390px", height: "844px", overflow: "hidden", background: "var(--bg-canvas)" }}>
      {/* Compact header */}
      <header style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "16px 20px", borderBottom: "1px solid var(--border-subtle)",
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Mark size={20} color="var(--accent)"/>
          <span style={{ fontFamily: "var(--font-display)", fontSize: 18, fontStyle: "italic" }}>Astrophoto</span>
        </div>
        <div style={{ width: 28, height: 28, borderRadius: "50%", background: "var(--accent)", color: "var(--accent-ink)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--font-display)", fontSize: 13 }}>M</div>
      </header>

      <section style={{ padding: "24px 20px 12px" }}>
        <div className="t-eyebrow">YOUR ARCHIVE</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 32, fontWeight: 400, margin: "6px 0 16px" }}>
          My <em>frames</em>
        </h1>
        <div style={{ display: "flex", gap: 16, fontFamily: "var(--font-mono)", fontSize: 11 }}>
          <div><span style={{ fontFamily: "var(--font-display)", fontSize: 20, color: "var(--fg-primary)" }}>42</span><span className="t-meta" style={{ marginLeft: 4 }}>PUB</span></div>
          <div><span style={{ fontFamily: "var(--font-display)", fontSize: 20, color: "var(--accent)" }}>3</span><span className="t-meta" style={{ marginLeft: 4 }}>DRAFTS</span></div>
          <div><span style={{ fontFamily: "var(--font-display)", fontSize: 20, color: "var(--fg-primary)" }}>318 h</span></div>
        </div>
      </section>

      {/* Drafts banner */}
      <div style={{ margin: "8px 20px", padding: 14, background: "var(--bg-warning-tint)", border: "1px solid var(--warning)", borderRadius: 2 }}>
        <div className="t-eyebrow" style={{ color: "var(--warning)" }}>● 3 DRAFTS WAITING</div>
        <p style={{ margin: "6px 0 10px", fontSize: 12, color: "var(--fg-secondary)" }}>NGC 7000, M27, Heart Nebula. Tap to continue.</p>
        <button className="btn btn-secondary btn-sm" style={{ width: "100%" }}>Resume drafts →</button>
      </div>

      {/* Filter chips horizontal */}
      <div style={{ display: "flex", gap: 6, padding: "8px 20px", overflowX: "auto" }}>
        {["All · 45", "Published", "Drafts · 3", "Sort: newest ▾"].map((t, i) => (
          <button key={i} className={"chip " + (i === 0 ? "chip-accent" : "")}>{t}</button>
        ))}
      </div>

      {/* List rows */}
      <div style={{ padding: "8px 20px" }}>
        {photos.slice(0, 5).map((p, i) => {
          const isDraft = i === 2;
          return (
            <div key={i} style={{ display: "flex", gap: 12, padding: "12px 0", borderBottom: "1px dashed var(--border-subtle)" }}>
              <div style={{ position: "relative", width: 64, height: 64, flexShrink: 0 }}>
                <Photo photo={p} style={{ position: "absolute", inset: 0 }}/>
                {isDraft && <div style={{ position: "absolute", inset: 0, background: "rgba(12,10,8,.4)", border: "1px dashed var(--warning)" }}/>}
              </div>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontFamily: "var(--font-display)", fontSize: 15, fontStyle: "italic", color: "var(--fg-primary)" }}>{p.target}</div>
                <div className="t-meta" style={{ marginTop: 4 }}>{p.integration} · {["14 MAR","09 MAR","02 MAR","27 FEB","18 FEB"][i]}</div>
                <div style={{ marginTop: 6 }}>
                  <span className={"chip " + (isDraft ? "" : "chip-accent")} style={isDraft ? { borderColor: "var(--warning)", color: "var(--warning)" } : {}}>
                    {isDraft ? "DRAFT" : `♡ ${[248, 412, 0, 187, 92][i]}`}
                  </span>
                </div>
              </div>
              <span style={{ color: "var(--fg-muted)", fontSize: 18 }}>⋯</span>
            </div>
          );
        })}
      </div>
    </div>
  );
};

/* ============================================================
   7. REPLACE FLOW — modal on photo detail
   ============================================================ */
window.ScreenReplaceModal = function ScreenReplaceModal({ marks }) {
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1080px", height: "780px", padding: "48px", background: "var(--bg-raised)", border: "1px solid var(--border-default)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "start", marginBottom: 24 }}>
        <div>
          <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● REPLACE IMAGE · KEEP CAPTION & METADATA</div>
          <h2 style={{ fontFamily: "var(--font-display)", fontSize: 32, fontWeight: 400, margin: "8px 0 0", fontStyle: "italic" }}>
            Swap a better master
          </h2>
          <p style={{ color: "var(--fg-secondary)", fontSize: 14, marginTop: 12, maxWidth: 640, lineHeight: 1.6 }}>
            Replace the image file while keeping the caption, comments, appreciations, and EXIF intact.
            For when you reprocess and want the new master to live at the same URL.
          </p>
        </div>
        <button className="btn btn-ghost" style={{ fontSize: 18, marginTop: -8 }}>✕</button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr auto 1fr", gap: 32, alignItems: "start" }}>
        {/* Current */}
        <div>
          <div className="t-label" style={{ marginBottom: 8 }}>CURRENT · POSTED 14 MAR</div>
          <div style={{ position: "relative", aspectRatio: "4/3" }}>
            <Photo photo={PHOTOS_P8B[7]} style={{ position: "absolute", inset: 0 }}/>
          </div>
          <div className="t-meta" style={{ marginTop: 8, display: "flex", justifyContent: "space-between" }}>
            <span>NGC7000_SHO_v2.jpg</span>
            <span>28.1 MB · 6248 × 4176</span>
          </div>
        </div>

        <div style={{
          alignSelf: "center", marginTop: 100,
          width: 32, height: 32, borderRadius: "50%",
          border: "1px solid var(--accent)", color: "var(--accent)",
          display: "flex", alignItems: "center", justifyContent: "center",
          fontSize: 16,
        }}>→</div>

        {/* New — drop zone */}
        <div>
          <div className="t-label" style={{ marginBottom: 8, color: "var(--accent)" }}>NEW MASTER</div>
          <div style={{
            aspectRatio: "4/3", border: "1px dashed var(--accent)", background: "var(--bg-accent-tint)",
            display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center",
            padding: 24,
          }}>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 22, fontStyle: "italic", color: "var(--accent)" }}>
              Drop the new file
            </div>
            <p style={{ color: "var(--fg-secondary)", fontSize: 12, marginTop: 8, maxWidth: 240 }}>
              JPG, PNG, or TIFF. Up to 64 MB. We'll regenerate thumbnails and keep your caption.
            </p>
            <button className="btn btn-secondary btn-sm" style={{ marginTop: 12 }}>Or pick a file…</button>
          </div>
          <div className="t-meta" style={{ marginTop: 8, color: "var(--fg-muted)" }}>
            EXIF FROM THE NEW FILE WILL BE READ; YOU CAN ACCEPT OR KEEP THE EXISTING METADATA.
          </div>
        </div>
      </div>

      <div style={{
        marginTop: 32, padding: 16, background: "var(--bg-warning-tint)", border: "1px solid var(--warning)",
        fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.6,
      }}>
        <strong style={{ color: "var(--warning)", fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.12em" }}>⚠ HEADS UP &nbsp;</strong>
        The previous file is removed from our servers. Comments and appreciations stay attached to this URL.
        Your followers see "REPROCESSED · 14 MAR → 02 MAY 2026" on the photo.
      </div>

      <div style={{ marginTop: 24, display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn btn-ghost">Cancel</button>
        <button className="btn btn-primary" disabled style={{ opacity: 0.5 }}>Replace image</button>
      </div>
    </div>
  );
};

/* ============================================================
   7B. REPLACE — dedicated page variant
   ============================================================ */
window.ScreenReplacePage = function ScreenReplacePage({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1100px", overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <section style={{ padding: "32px 64px 24px", borderBottom: "1px solid var(--border-subtle)" }}>
        <div className="t-eyebrow">REPLACE IMAGE · NGC 7000 · NORTH AMERICA</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 44, fontWeight: 400, margin: "8px 0 0", lineHeight: 1 }}>
          Reprocess & <em>swap the master</em>
        </h1>
        <div style={{ display: "flex", gap: 0, marginTop: 24, fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.12em", textTransform: "uppercase" }}>
          {[["01", "UPLOAD NEW", "active"], ["02", "ACCEPT EXIF?", ""], ["03", "CONFIRM SWAP", ""]].map(([n, l, s]) => (
            <div key={n} style={{
              flex: 1, padding: "14px 0", borderTop: `2px solid ${s ? "var(--accent)" : "var(--border-default)"}`,
              color: s ? "var(--fg-primary)" : "var(--fg-muted)",
              display: "flex", gap: 12, alignItems: "center",
            }}>
              <span style={{ color: s ? "var(--accent)" : "var(--fg-faint)" }}>{n}</span>
              <span>{l}</span>
            </div>
          ))}
        </div>
      </section>

      <section style={{ padding: "40px 64px", display: "grid", gridTemplateColumns: "1fr 1fr", gap: 48 }}>
        <div>
          <div className="t-label" style={{ marginBottom: 12 }}>CURRENT MASTER · POSTED 14 MAR 2026</div>
          <div style={{ position: "relative", aspectRatio: "4/3" }}>
            <Photo photo={PHOTOS_P8B[7]} style={{ position: "absolute", inset: 0 }}/>
            <div style={{ position: "absolute", left: 12, top: 12, padding: "4px 8px", background: "rgba(12,10,8,.85)", border: "1px solid var(--border-default)", fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--fg-muted)", letterSpacing: "0.08em" }}>
              CURRENT
            </div>
          </div>
          <table className="exif" style={{ marginTop: 16 }}>
            <tbody>
              <tr><th>FILENAME</th><td>NGC7000_SHO_v2.jpg</td></tr>
              <tr><th>DIMENSIONS</th><td>6248 × 4176</td></tr>
              <tr><th>SIZE</th><td>28.1 MB</td></tr>
              <tr><th>APPRECIATIONS</th><td>248 · KEPT</td></tr>
              <tr><th>COMMENTS</th><td>17 · KEPT</td></tr>
            </tbody>
          </table>
        </div>
        <div>
          <div className="t-label" style={{ marginBottom: 12, color: "var(--accent)" }}>NEW MASTER</div>
          <div style={{
            aspectRatio: "4/3", border: "1px dashed var(--accent)", background: "var(--bg-accent-tint)",
            display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center",
          }}>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 28, fontStyle: "italic", color: "var(--accent)" }}>Drop file here</div>
            <p style={{ color: "var(--fg-secondary)", fontSize: 13, marginTop: 12, maxWidth: 280 }}>
              We'll compare its EXIF against the current record on the next step.
            </p>
            <button className="btn btn-secondary" style={{ marginTop: 16 }}>Or pick a file…</button>
          </div>
          <div style={{ marginTop: 16, padding: 16, border: "1px dashed var(--border-default)", fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.6 }}>
            <div className="t-label" style={{ color: "var(--accent)", marginBottom: 6 }}>WHEN YOU REPROCESS</div>
            Followers see a small "REPROCESSED · 02 MAY 2026" mark on the photo.
            The old file is removed from our servers. The URL doesn't change.
          </div>
        </div>
      </section>

      <section style={{ padding: "0 64px", display: "flex", gap: 12, justifyContent: "flex-end" }}>
        <button className="btn btn-ghost">← Back to photo</button>
        <button className="btn btn-secondary">Save as draft replacement</button>
        <button className="btn btn-primary" disabled style={{ opacity: 0.5 }}>Continue → check EXIF</button>
      </section>
    </div>
  );
};

/* ============================================================
   8. POLISH 8.5 — eyebrow, FollowButton, untitled fallback,
                   mobile AppreciateButton
   ============================================================ */
window.ScreenPolish = function ScreenPolish({ marks }) {
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: "1440px", height: "1500px", padding: "64px", overflow: "hidden" }}>
      <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● POLISH · PHASE 8.5 · COPY & MICRO-COMPONENTS</div>
      <h1 style={{ fontFamily: "var(--font-display)", fontSize: 56, fontWeight: 400, margin: "8px 0 0", lineHeight: 1 }}>
        Four <em>small fixes</em>, four big effects.
      </h1>
      <p style={{ color: "var(--fg-secondary)", fontSize: 15, marginTop: 16, maxWidth: 720, lineHeight: 1.6 }}>
        Items that don't belong on their own artboards but matter to feel.
        Each is shown before / after, with the chosen behaviour annotated.
      </p>

      {/* 1. Eyebrow on logged-in home */}
      <div style={{ marginTop: 56 }}>
        <div className="t-eyebrow">01 · CONTEXT-AWARE EYEBROW · LOGGED-IN HOME</div>
        <p style={{ color: "var(--fg-muted)", fontSize: 13, margin: "6px 0 16px", maxWidth: 640 }}>
          Public eyebrow says the date. Logged-in eyebrow names what's actually below: the people you follow, and how many new frames they posted.
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
          <BeforeAfter label="BEFORE">
            <div className="t-eyebrow" style={{ marginBottom: 8 }}>WELCOME BACK · 02 MAY 2026 · NEW MOON IN 6 DAYS</div>
            <h2 style={{ fontFamily: "var(--font-display)", fontSize: 36, fontWeight: 400, margin: 0 }}>Good evening, <em>Marie</em>.</h2>
          </BeforeAfter>
          <BeforeAfter label="AFTER" highlight>
            <div className="t-eyebrow" style={{ marginBottom: 8, color: "var(--accent)" }}>● FROM THE 28 PHOTOGRAPHERS YOU FOLLOW · 12 NEW</div>
            <h2 style={{ fontFamily: "var(--font-display)", fontSize: 36, fontWeight: 400, margin: 0 }}>Good evening, <em>Marie</em>.</h2>
            <p style={{ marginTop: 12, color: "var(--fg-secondary)", fontSize: 13 }}>2 May 2026 · clear skies tonight in Provence · new moon in 6 days.</p>
          </BeforeAfter>
        </div>
      </div>

      {/* 2. FollowButton on photo detail */}
      <div style={{ marginTop: 56 }}>
        <div className="t-eyebrow">02 · FOLLOW BUTTON · PHOTO DETAIL HEADER</div>
        <p style={{ color: "var(--fg-muted)", fontSize: 13, margin: "6px 0 16px", maxWidth: 640 }}>
          Currently a navigation link to the user's profile. Now: a real follow toggle, with three states.
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16 }}>
          {[
            ["NOT FOLLOWING", "default", "Follow"],
            ["FOLLOWING · DEFAULT", "following", "✓ Following"],
            ["FOLLOWING · HOVER", "unfollow", "Unfollow?"],
          ].map(([state, kind, label], i) => (
            <div key={i} style={{ padding: 24, border: "1px solid var(--border-subtle)", background: "var(--bg-raised)" }}>
              <div className="t-meta" style={{ color: "var(--fg-muted)", marginBottom: 16 }}>{state}</div>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <div style={{ width: 44, height: 44, borderRadius: "50%", background: "var(--accent)", color: "var(--accent-ink)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "var(--font-display)", fontSize: 18 }}>M</div>
                <div style={{ flex: 1 }}>
                  <div style={{ fontFamily: "var(--font-display)", fontSize: 16, fontStyle: "italic" }}>Marie Dubois</div>
                  <div className="t-meta">@MARIE.DUBOIS · 2,841 FOLLOWERS</div>
                </div>
                {kind === "default" && <button className="btn btn-primary btn-sm">{label}</button>}
                {kind === "following" && <button className="btn btn-secondary btn-sm" style={{ borderColor: "var(--accent-dim)", color: "var(--accent)" }}>{label}</button>}
                {kind === "unfollow" && <button className="btn btn-secondary btn-sm" style={{ borderColor: "var(--danger)", color: "var(--danger)" }}>{label}</button>}
              </div>
            </div>
          ))}
        </div>
        <div style={{ marginTop: 16, padding: 12, background: "var(--bg-base)", border: "1px dashed var(--border-default)", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--fg-muted)", letterSpacing: "0.06em", lineHeight: 1.7 }}>
          BEHAVIOUR · CLICK NAME → PROFILE. CLICK BUTTON → TOGGLE FOLLOW.<br/>
          ANIMATION · ON FOLLOW, BUTTON FILLS THEN SETTLES INTO ✓ STATE OVER 240MS.
        </div>
      </div>

      {/* 3. Untitled fallback */}
      <div style={{ marginTop: 56 }}>
        <div className="t-eyebrow">03 · UNTITLED PHOTO · FILENAME FALLBACK</div>
        <p style={{ color: "var(--fg-muted)", fontSize: 13, margin: "6px 0 16px", maxWidth: 640 }}>
          Photos without a target name fall back to filename, italicised + muted, with a small "UNTITLED" tag. We never hide the caption block — silence is suspicious.
        </p>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
          <BeforeAfter label="BEFORE · CAPTION HIDDEN">
            <div style={{ position: "relative", aspectRatio: "16/10" }}>
              <Photo photo={PHOTOS_P8B[6]} style={{ position: "absolute", inset: 0 }}/>
            </div>
            <div style={{ padding: "16px 0", color: "var(--fg-faint)", fontSize: 12, fontStyle: "italic" }}>(no caption block at all)</div>
          </BeforeAfter>
          <BeforeAfter label="AFTER · FILENAME ITALIC + UNTITLED CHIP" highlight>
            <div style={{ position: "relative", aspectRatio: "16/10" }}>
              <Photo photo={PHOTOS_P8B[6]} style={{ position: "absolute", inset: 0 }}/>
            </div>
            <div style={{ padding: "16px 0" }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
                <span style={{ fontFamily: "var(--font-display)", fontSize: 20, fontStyle: "italic", color: "var(--fg-muted)" }}>
                  IC434_horsehead_v3.tif
                </span>
                <span className="chip" style={{ borderStyle: "dashed" }}>UNTITLED</span>
              </div>
              <div className="t-meta" style={{ marginTop: 6 }}>COMETCHASER_2024 · 8H 48M · ASI294MC</div>
            </div>
          </BeforeAfter>
        </div>
      </div>

      {/* 4. Mobile AppreciateButton */}
      <div style={{ marginTop: 56 }}>
        <div className="t-eyebrow">04 · MOBILE APPRECIATE BUTTON · STICKY BAR</div>
        <p style={{ color: "var(--fg-muted)", fontSize: 13, margin: "6px 0 16px", maxWidth: 640 }}>
          Replaces the inert placeholder. 44px hit target, count visible at rest, tap fills the heart and increments. Long-press shows recent appreciators.
        </p>
        <div style={{ display: "flex", gap: 24, alignItems: "flex-end" }}>
          {[
            ["DEFAULT · 247 APPRECIATIONS", false],
            ["ACTIVE · YOU APPRECIATED · 248", true],
          ].map(([state, on], i) => (
            <div key={i} style={{ width: 390 }}>
              <div className="t-meta" style={{ marginBottom: 8, color: "var(--fg-muted)" }}>{state}</div>
              {/* fake mobile sticky bar */}
              <div style={{
                height: 64, padding: "0 16px",
                background: "var(--bg-overlay)",
                backdropFilter: "blur(12px)",
                border: "1px solid var(--border-subtle)",
                borderRadius: 4,
                display: "flex", alignItems: "center", gap: 12,
              }}>
                <div style={{
                  display: "flex", alignItems: "center", gap: 8,
                  height: 44, padding: "0 16px", minWidth: 96,
                  background: on ? "var(--bg-accent-tint)" : "transparent",
                  border: "1px solid " + (on ? "var(--accent)" : "var(--border-default)"),
                  borderRadius: 999,
                }}>
                  <svg width="18" height="18" viewBox="0 0 24 24" fill={on ? "var(--accent)" : "none"} stroke={on ? "var(--accent)" : "var(--fg-secondary)"} strokeWidth="1.6">
                    <path d="M12 21s-7-4.5-9-9a5 5 0 0 1 9-3 5 5 0 0 1 9 3c-2 4.5-9 9-9 9z"/>
                  </svg>
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: on ? "var(--accent)" : "var(--fg-secondary)" }}>{on ? 248 : 247}</span>
                </div>
                <button className="btn btn-ghost btn-sm" style={{ height: 44, color: "var(--fg-secondary)" }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
                  &nbsp;17
                </button>
                <div style={{ flex: 1 }}/>
                <button className="btn btn-secondary btn-sm" style={{ height: 44 }}>Save</button>
                <button className="btn btn-secondary btn-sm" style={{ height: 44, width: 44, padding: 0 }}>⤴</button>
              </div>
              <div className="t-meta" style={{ marginTop: 8, color: "var(--fg-muted)" }}>
                MIN HIT · 44 × 44 · LONG-PRESS REVEALS RECENT APPRECIATORS
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

const BeforeAfter = ({ label, children, highlight }) => (
  <div style={{
    padding: 24, background: "var(--bg-raised)",
    border: "1px solid " + (highlight ? "var(--accent-dim)" : "var(--border-subtle)"),
    position: "relative",
  }}>
    <div className="t-meta" style={{ color: highlight ? "var(--accent)" : "var(--fg-muted)", marginBottom: 16 }}>{label}</div>
    {children}
  </div>
);
