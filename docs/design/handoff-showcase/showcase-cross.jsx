/* =============================================================
   ASTROPHOTO · SHOWCASE — CROSS-CUTTING
   Tier upgrade modal · plate-solve forward-compat · error states
   ============================================================= */

const Eyebrow4 = window.AP_Eyebrow;
const Display4 = window.AP_Display;
const PageHeader4 = window.AP_PageHeader;

/* ============================================================
   X1 — TIER UPGRADE PROMPT
   Triggered when free user tries to upload >50 MB
   ============================================================ */
window.ScreenTierUpgrade = function ({ marks }) {
  return (
    <div className="screen" style={{ width: 720, height: 720, padding: 0, background: "var(--bg-raised)", border: "1px solid var(--border-default)", overflow: "hidden" }}>
      <div style={{ padding: "24px 32px", borderBottom: "1px solid var(--border-subtle)", display: "flex", justifyContent: "space-between", alignItems: "start" }}>
        <div>
          <Eyebrow4 accent>● UPGRADE · UNLOCK 200 MB FRAMES</Eyebrow4>
          <Display4 size={32} style={{ marginTop: 6 }}>Your <em>master</em> doesn't fit</Display4>
        </div>
        <button className="btn btn-ghost" style={{ fontSize: 18, marginTop: -8 }}>✕</button>
      </div>

      <div style={{ padding: "24px 32px" }}>
        {/* Diagnostic */}
        <div style={{
          padding: 16, background: "var(--bg-warning-tint)",
          border: "1px solid var(--warning)",
          fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.7,
        }}>
          <div className="t-eyebrow" style={{ color: "var(--warning)", marginBottom: 8 }}>● BLOCKED AT PRESIGN STEP</div>
          File: <span style={{ color: "var(--fg-primary)" }}>NGC7000_SHO_v3.tif</span><br/>
          Size: <span style={{ color: "var(--warning)" }}>137.4 MB</span> · your tier cap: <span style={{ color: "var(--fg-primary)" }}>50 MB</span><br/>
          Result: <span style={{ color: "var(--warning)" }}>S3 PUT would 400 — bandwidth never starts.</span>
        </div>

        {/* Tier comparison */}
        <div style={{
          marginTop: 24, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12,
        }}>
          <div style={{ padding: 20, border: "1px solid var(--border-default)", background: "var(--bg-base)" }}>
            <div className="t-label">YOU · FREE</div>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 36, fontStyle: "italic", marginTop: 8 }}>50 <span className="t-mono" style={{ fontSize: 14 }}>MB / FRAME</span></div>
            <ul style={{ margin: "16px 0 0", padding: 0, listStyle: "none", fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.9 }}>
              <li>✓ Unlimited frames</li>
              <li>✓ Hero page · /u/&lt;handle&gt;</li>
              <li>✓ Discovery & search</li>
              <li style={{ color: "var(--fg-faint)" }}>— TIFF over 50 MB blocked</li>
            </ul>
          </div>
          <div style={{ padding: 20, border: "1px solid var(--accent)", background: "var(--bg-accent-tint)", position: "relative" }}>
            <div style={{ position: "absolute", top: -10, right: 16, padding: "2px 8px", background: "var(--accent)", color: "var(--accent-ink)", fontFamily: "var(--font-mono)", fontSize: 10, letterSpacing: "0.1em" }}>RECOMMENDED</div>
            <div className="t-label" style={{ color: "var(--accent)" }}>SUBSCRIBER</div>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 36, fontStyle: "italic", marginTop: 8, color: "var(--accent)" }}>200 <span className="t-mono" style={{ fontSize: 14 }}>MB / FRAME</span></div>
            <ul style={{ margin: "16px 0 0", padding: 0, listStyle: "none", fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.9 }}>
              <li>✓ <strong style={{ color: "var(--fg-primary)" }}>200 MB</strong> · 16-bit TIFF welcome</li>
              <li>✓ Original master kept · CDN serves a derived 4096 px JPEG</li>
              <li>✓ Priority transform queue</li>
              <li>✓ Everything in Free</li>
            </ul>
          </div>
        </div>

        {/* Actions */}
        <div style={{ marginTop: 24, display: "flex", gap: 8, justifyContent: "flex-end", alignItems: "center" }}>
          <span className="t-meta" style={{ color: "var(--fg-muted)", marginRight: "auto" }}>
            BILLING UI SHIPS LATER · TIER FLAG ENFORCED NOW
          </span>
          <button className="btn btn-ghost">Replace with smaller file</button>
          <button className="btn btn-primary">Upgrade to subscriber</button>
        </div>
      </div>
    </div>
  );
};

/* ============================================================
   X2 — PLATE-SOLVE FORWARD-COMPAT NOTE
   ============================================================ */
window.ScreenPlateSolveNote = function ({ marks }) {
  return (
    <div className="screen" style={{ width: 1080, height: 700, padding: "48px 64px", overflow: "hidden" }}>
      <Eyebrow4 accent>● FORWARD-COMPAT · PLATE-SOLVE-READY DAY ONE</Eyebrow4>
      <Display4 size={44} style={{ marginTop: 8 }}>What ships <em>now</em>, what slides in <em>later</em></Display4>
      <p style={{ color: "var(--fg-muted)", fontSize: 14, marginTop: 12, maxWidth: 720, lineHeight: 1.7 }}>
        The data model accepts plate-solved targets from day one — but the writer is gated.
        Every discovery query reads <code className="t-mono" style={{ color: "var(--accent)" }}>photo_targets</code> regardless of source, so when astrometry comes online, target pages just fill up.
      </p>

      <div style={{ marginTop: 32, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
        <div style={{ padding: 24, border: "1px solid var(--accent-dim)", background: "var(--bg-raised)" }}>
          <div className="t-label" style={{ color: "var(--accent)" }}>● P1–P3 · NOW</div>
          <div style={{
            marginTop: 12, fontFamily: "var(--font-display)", fontSize: 22, fontStyle: "italic",
          }}>source = <span style={{ color: "var(--accent)" }}>'manual'</span></div>
          <ul style={{ margin: "16px 0 0", padding: 0, listStyle: "none", fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.9 }}>
            <li>✓ User picks target in upload-verify (autocomplete)</li>
            <li>✓ Free-text aliases match against <code className="t-mono">targets.aliases</code></li>
            <li>✓ Row in <code className="t-mono">photo_targets</code> with <code className="t-mono">is_primary=true</code></li>
            <li>✓ <code className="t-mono">confidence = NULL</code></li>
          </ul>
        </div>

        <div style={{ padding: 24, border: "1px dashed var(--border-default)", background: "var(--bg-base)" }}>
          <div className="t-label" style={{ color: "var(--fg-muted)" }}>● FUTURE · DROPPED IN LATER</div>
          <div style={{
            marginTop: 12, fontFamily: "var(--font-display)", fontSize: 22, fontStyle: "italic",
            color: "var(--fg-muted)",
          }}>source = <span style={{ color: "var(--fg-primary)" }}>'plate_solve'</span></div>
          <ul style={{ margin: "16px 0 0", padding: 0, listStyle: "none", fontSize: 13, color: "var(--fg-muted)", lineHeight: 1.9 }}>
            <li>○ Astrometry job writes 1..N rows per photo</li>
            <li>○ <code className="t-mono">confidence</code> populated · 0–1</li>
            <li>○ UI shows secondary targets ("also visible: …")</li>
            <li>○ <strong>Zero schema churn</strong> · queries unchanged</li>
          </ul>
        </div>
      </div>

      <div style={{
        marginTop: 24, padding: 16, background: "var(--bg-base)",
        border: "1px dashed var(--border-default)", fontFamily: "var(--font-mono)", fontSize: 12,
        color: "var(--fg-secondary)", lineHeight: 1.7,
      }}>
        <span className="t-eyebrow" style={{ color: "var(--accent)" }}>● TARGET PAGE QUERY · UNCHANGED ACROSS ERAS</span><br/><br/>
        SELECT p.* FROM photos p<br/>
        &nbsp;&nbsp;JOIN photo_targets pt ON pt.photo_id = p.id<br/>
        &nbsp;&nbsp;WHERE pt.target_id = $1<br/>
        &nbsp;&nbsp;AND p.published_at IS NOT NULL<br/>
        &nbsp;&nbsp;<span style={{ color: "var(--fg-muted)" }}>-- pt.source ∈ ('manual', 'plate_solve') · we read both</span><br/>
        &nbsp;&nbsp;ORDER BY p.published_at DESC, p.id DESC;
      </div>
    </div>
  );
};

/* ============================================================
   X3 — ERROR STATES INVENTORY
   ============================================================ */
window.ScreenErrorStates = function ({ marks }) {
  return (
    <div className="screen" style={{ width: 1440, height: 1100, padding: "48px 64px", overflow: "hidden" }}>
      <Eyebrow4>● ERROR STATES · APPERROR VARIANTS</Eyebrow4>
      <Display4 size={44} style={{ marginTop: 8 }}>How <em>refusal</em> looks</Display4>
      <p style={{ color: "var(--fg-muted)", fontSize: 14, marginTop: 12, maxWidth: 720, lineHeight: 1.7 }}>
        Every refusal is a sentence, not a stack trace. Backend variant maps to one HTTP code and one user-facing chip + sentence.
        Inputs that caused it stay highlighted; the action is recoverable.
      </p>

      <div style={{ marginTop: 32, display: "grid", gridTemplateColumns: "200px 1fr 1fr", gap: 16 }}>
        <Header3>VARIANT</Header3>
        <Header3>USER-FACING</Header3>
        <Header3>RECOVERY</Header3>

        <ErrRow
          variant="QuotaExceeded · 413"
          chip="● 50 MB / FILE · FREE TIER"
          chipColor="var(--warning)"
          msg="This frame is 137 MB. Your tier ceiling is 50 MB."
          recovery="Upgrade · Pick smaller file · Cancel"
        />
        <ErrRow
          variant="PayloadTooLarge · 413"
          chip="● S3 PUT REJECTED"
          chipColor="var(--danger)"
          msg="Upload stopped before it started — S3 returned 400 on Content-Length."
          recovery="Retry with smaller file · Upgrade tier"
        />
        <ErrRow
          variant="Conflict · 409"
          chip="● HANDLE TAKEN"
          chipColor="var(--warning)"
          msg="@marie-dubois is taken. Try @marie.dubois or @marie-d."
          recovery="Pick different handle"
        />
        <ErrRow
          variant="Conflict · 409 · DUP HASH"
          chip="● ALREADY UPLOADED"
          chipColor="var(--warning)"
          msg="You've uploaded this exact file before — see /u/marie-dubois/p/X8K2 · 14 Mar."
          recovery="Open existing · Replace existing · Force new (subscriber)"
        />
        <ErrRow
          variant="RateLimited · 429"
          chip="● SLOW DOWN"
          chipColor="var(--warning)"
          msg="You've hit the 30 likes / minute limit. Try again in 12 s."
          recovery="(automatic — UI counts down)"
        />
        <ErrRow
          variant="Magic-byte mismatch · 400"
          chip="● FILE TYPE LIES"
          chipColor="var(--danger)"
          msg="The file claims to be JPEG but its magic bytes say something else."
          recovery="Re-export from your processor"
        />
        <ErrRow
          variant="PendingFinalizeStuck · 408"
          chip="● PUT NEVER COMPLETED"
          chipColor="var(--warning)"
          msg="Browser dropped before S3 confirmed. The orphan reaper will clean this up."
          recovery="Retry upload · Discard"
        />
        <ErrRow
          variant="UnsupportedFormat · 400"
          chip="● FITS / RAW NOT YET"
          chipColor="var(--fg-muted)"
          msg="We only accept JPEG, PNG, and TIFF for now. FITS support is on the roadmap."
          recovery="Export to TIFF and retry"
        />
      </div>

      <div className="t-meta" style={{ marginTop: 24, color: "var(--fg-muted)", lineHeight: 1.7 }}>
        ● ALL VARIANTS IMPL <code className="t-mono" style={{ color: "var(--accent)" }}>INTORESPONSE</code>.
        BODY · <code className="t-mono">{`{code, message, hint?, retry_after?}`}</code> · NEVER THE STACK TRACE.
      </div>
    </div>
  );
};

function Header3({ children }) {
  return (
    <div className="t-eyebrow" style={{ paddingBottom: 8, borderBottom: "1px solid var(--border-default)" }}>{children}</div>
  );
}
function ErrRow({ variant, chip, chipColor, msg, recovery }) {
  return (
    <>
      <div style={{ padding: "14px 0", borderBottom: "1px dashed var(--border-subtle)" }}>
        <div className="t-mono" style={{ fontSize: 12, color: "var(--fg-secondary)" }}>{variant}</div>
      </div>
      <div style={{ padding: "14px 0", borderBottom: "1px dashed var(--border-subtle)" }}>
        <div className="chip" style={{ borderColor: chipColor, color: chipColor }}>{chip}</div>
        <div style={{
          marginTop: 8, fontFamily: "var(--font-display)", fontSize: 15, fontStyle: "italic",
          color: "var(--fg-primary)", lineHeight: 1.5,
        }}>"{msg}"</div>
      </div>
      <div style={{ padding: "14px 0", borderBottom: "1px dashed var(--border-subtle)" }}>
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {recovery.split(" · ").map((r, i) => (
            <span key={r} className="chip" style={i === 0 ? { borderColor: "var(--accent)", color: "var(--accent)" } : {}}>{r}</span>
          ))}
        </div>
      </div>
    </>
  );
}

/* ============================================================
   X4 — URL MAP & ROUTE INVENTORY
   ============================================================ */
window.ScreenRouteMap = function ({ marks }) {
  return (
    <div className="screen" style={{ width: 1440, height: 1000, padding: "48px 64px", overflow: "hidden" }}>
      <Eyebrow4>● URL MAP · BEFORE / AFTER · WHAT 301S</Eyebrow4>
      <Display4 size={44} style={{ marginTop: 8 }}>The <em>shape</em> of the site</Display4>

      <div style={{ marginTop: 32, display: "grid", gridTemplateColumns: "1.2fr 1fr 1.2fr", gap: 16 }}>
        <Header3>BEFORE</Header3>
        <Header3 />
        <Header3>AFTER · CANONICAL</Header3>

        {[
          ["/photo/<uuid>",     "301",  "/u/<handle>/p/<short-id>"],
          ["/u/<old-handle>/*", "301",  "/u/<new-handle>/* · 90-day cooldown"],
          ["/upload",           "—",    "/upload · 3-step wizard rebuilt"],
          ["—",                 "NEW",  "/u/<handle> · hero page"],
          ["—",                 "NEW",  "/u/<handle>/p/<short-id> · photo · lightbox-overlay-routed"],
          ["—",                 "NEW",  "/explore"],
          ["—",                 "NEW",  "/t/<slug> · target"],
          ["—",                 "NEW",  "/tag/<slug>"],
          ["—",                 "NEW",  "/equip/<kind>/<slug>"],
          ["—",                 "NEW",  "/c/<category>"],
          ["—",                 "NEW",  "/search?q="],
          ["—",                 "NEW",  "/account/handle/setup · backfill prompt for legacy users"],
        ].map(([before, mid, after]) => (
          <React.Fragment key={after}>
            <div style={{ padding: "12px 0", borderBottom: "1px dashed var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: 13, color: before === "—" ? "var(--fg-faint)" : "var(--fg-secondary)" }}>
              {before}
            </div>
            <div style={{ padding: "12px 0", borderBottom: "1px dashed var(--border-subtle)", textAlign: "center" }}>
              <span className="chip" style={{
                borderColor: mid === "NEW" ? "var(--accent)" : (mid === "301" ? "var(--warning)" : "var(--border-default)"),
                color: mid === "NEW" ? "var(--accent)" : (mid === "301" ? "var(--warning)" : "var(--fg-muted)"),
              }}>{mid === "—" ? "kept" : mid}</span>
            </div>
            <div style={{ padding: "12px 0", borderBottom: "1px dashed var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--fg-primary)" }}>
              {after}
            </div>
          </React.Fragment>
        ))}
      </div>
    </div>
  );
};
