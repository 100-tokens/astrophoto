/* ===== ASTROPHOTO PHASE 8 — Settings v2, Password reset, 2FA, Sessions, Deletion =====
   Sectioned scroll with sticky left rail. Matches the existing Settings vocabulary.
*/

const PHOTOS_P8 = window.PHOTOS;

/* ============================================================
   SHARED — Settings shell
   ============================================================ */
function SettingsShell({ active = "PROFILE", marks, children, deletionGrace = false }) {
  const AppHeader = window.AppHeader;
  const items = [
    ["PROFILE", ""],
    ["EQUIPMENT", ""],
    ["NOTIFICATIONS", ""],
    ["EMAIL & SECURITY", ""],
    ["APPEARANCE", ""],
    ["SESSIONS", ""],
    ["DELETE ACCOUNT", "danger"],
  ];
  return (
    <div className="screen" style={{ width: "1440px", overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      {deletionGrace && (
        <div style={{
          padding: "12px 64px", background: "var(--bg-danger-tint)",
          borderBottom: "1px solid var(--danger)",
          fontFamily: "var(--font-mono)", fontSize: 12,
          display: "flex", justifyContent: "space-between", alignItems: "center",
          color: "var(--fg-primary)",
        }}>
          <span>
            <span style={{ color: "var(--danger)" }}>● ACCOUNT MARKED FOR DELETION</span>
            <span style={{ marginLeft: 24, color: "var(--fg-secondary)" }}>
              Permanent removal in <strong style={{ color: "var(--fg-primary)" }}>6 days, 14 hours</strong> ·
              42 frames will be erased
            </span>
          </span>
          <a style={{ color: "var(--accent)", textDecoration: "underline" }}>Cancel deletion</a>
        </div>
      )}
      <section style={{ padding: "40px 64px 0" }}>
        <div className="t-eyebrow">PREFERENCES</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 0" }}>
          Account <em>settings</em>
        </h1>
      </section>
      <section style={{ display: "grid", gridTemplateColumns: "240px 1fr", gap: 64, padding: "40px 64px" }}>
        <nav style={{ display: "flex", flexDirection: "column", gap: 4, fontFamily: "var(--font-mono)", fontSize: 12, position: "sticky", top: 0, alignSelf: "start" }}>
          {items.map(([n, s]) => (
            <a key={n} style={{
              padding: "10px 12px",
              letterSpacing: "0.12em",
              color: active === n ? "var(--accent)" : s === "danger" ? "var(--danger)" : "var(--fg-muted)",
              borderLeft: active === n ? "1px solid var(--accent)" : "1px solid transparent",
              background: active === n ? "var(--bg-accent-tint)" : "transparent",
            }}>{n}</a>
          ))}
          <div style={{ marginTop: 24, padding: 12, fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--fg-faint)", letterSpacing: "0.1em", borderTop: "1px solid var(--border-subtle)" }}>
            ALL CHANGES AUTOSAVE<br/>EXCEPT EMAIL · PASSWORD<br/>· 2FA · DELETION
          </div>
        </nav>
        <div style={{ maxWidth: 720 }}>
          {children}
        </div>
      </section>
    </div>
  );
}

const P8Section = ({ title, desc, children, danger = false }) => (
  <div style={{ paddingBottom: 40, borderBottom: "1px solid var(--border-subtle)", marginBottom: 40 }}>
    <h2 style={{
      fontFamily: "var(--font-display)", fontSize: 26, fontWeight: 400, margin: "0 0 4px",
      fontStyle: "italic",
      color: danger ? "var(--danger)" : "var(--fg-primary)",
    }}>{title}</h2>
    <p style={{ color: "var(--fg-muted)", fontSize: 13, margin: "0 0 24px", maxWidth: 560 }}>{desc}</p>
    <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>{children}</div>
  </div>
);

const P8Row = ({ label, hint, children }) => (
  <div style={{ display: "grid", gridTemplateColumns: "160px 1fr", gap: 24, alignItems: "start" }}>
    <div style={{ paddingTop: 10 }}>
      <div className="t-label">{label}</div>
      {hint && <div className="t-meta" style={{ marginTop: 6, lineHeight: 1.5 }}>{hint}</div>}
    </div>
    <div>{children}</div>
  </div>
);

/* ============================================================
   1A. SETTINGS — extended (Email & Security focus)
   ============================================================ */
window.ScreenSettingsExtended = function ScreenSettingsExtended({ marks }) {
  return (
    <SettingsShell active="EMAIL & SECURITY" marks={marks}>
      <P8Section title="Sign-in identity" desc="Your email is how you sign in and how we reach you. We never publish it.">
        <P8Row label="EMAIL" hint="Verified · primary">
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <input className="input" defaultValue="marie.dubois@example.fr" style={{ flex: 1 }}/>
            <button className="btn btn-secondary">Change…</button>
          </div>
          <div className="t-meta" style={{ marginTop: 8, color: "var(--fg-muted)" }}>
            ● VERIFIED 12 JAN 2026 · LAST USED FROM SAINT-ÉTIENNE-LES-ORGUES
          </div>
        </P8Row>
        <P8Row label="PASSWORD" hint="Last changed 4 months ago.">
          <div style={{ display: "flex", gap: 8 }}>
            <input className="input" type="password" defaultValue="••••••••••••" style={{ flex: 1 }} disabled/>
            <button className="btn btn-secondary">Change…</button>
          </div>
        </P8Row>
      </P8Section>

      <P8Section title="Two-factor authentication" desc="A six-digit code from an authenticator app, in addition to your password. Recommended.">
        <div style={{
          padding: 20, background: "var(--bg-success-tint)",
          border: "1px solid var(--success)", borderRadius: 4,
          display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 20, alignItems: "center",
        }}>
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--success)" strokeWidth="1.4">
            <path d="M12 2L4 6v6c0 5 3.5 9 8 10 4.5-1 8-5 8-10V6l-8-4z"/>
            <path d="M9 12l2 2 4-4"/>
          </svg>
          <div>
            <div style={{ fontFamily: "var(--font-display)", fontSize: 17, fontStyle: "italic", color: "var(--fg-primary)" }}>
              2FA is on
            </div>
            <div className="t-meta" style={{ marginTop: 4 }}>
              AUTHY · ADDED 02 FEB 2026 · 8 OF 10 BACKUP CODES UNUSED
            </div>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="btn btn-ghost btn-sm">View backup codes</button>
            <button className="btn btn-danger btn-sm">Disable</button>
          </div>
        </div>
      </P8Section>

      <P8Section title="Active sessions" desc="Where you're signed in. Revoke anything you don't recognise.">
        {[
          ["This device", "Firefox 122 · macOS 14", "Saint-Étienne-les-Orgues, FR · 81.92.14.21", "Active now", true],
          ["iPhone", "Safari · iOS 17", "Marseille, FR · 90.84.211.07", "2 hours ago", false],
          ["Desktop", "Chrome 121 · Linux", "Saint-Étienne-les-Orgues, FR · 81.92.14.21", "Yesterday", false],
        ].map(([title, device, where, when, current], i) => (
          <div key={i} style={{
            display: "grid", gridTemplateColumns: "auto 1fr auto auto", gap: 20, alignItems: "center",
            padding: "16px 20px", border: "1px solid var(--border-subtle)", borderRadius: 4,
            background: current ? "var(--bg-accent-tint)" : "transparent",
          }}>
            <div style={{
              width: 8, height: 8, borderRadius: "50%",
              background: current ? "var(--accent)" : "var(--border-strong)",
            }}/>
            <div>
              <div style={{ fontFamily: "var(--font-display)", fontSize: 16, fontStyle: "italic" }}>
                {title}{current && <span className="t-meta" style={{ marginLeft: 12, color: "var(--accent)" }}>· this device</span>}
              </div>
              <div className="t-meta" style={{ marginTop: 4 }}>{device.toUpperCase()}</div>
              <div className="t-meta" style={{ marginTop: 2 }}>{where.toUpperCase()}</div>
            </div>
            <div className="t-meta" style={{ color: "var(--fg-secondary)" }}>{when.toUpperCase()}</div>
            {!current && <button className="btn btn-danger btn-sm">Revoke</button>}
            {current && <span style={{ width: 64 }}/>}
          </div>
        ))}
        <button className="btn btn-secondary" style={{ alignSelf: "flex-start" }}>Sign out of all other sessions</button>
      </P8Section>

      <P8Section title="Delete account" desc="Permanently delete your account and all 42 frames. There's a 7-day grace period — you can sign in to cancel any time before then." danger>
        <div style={{
          padding: 20, background: "var(--bg-danger-tint)",
          border: "1px solid var(--danger)", borderRadius: 4,
        }}>
          <p style={{ margin: 0, color: "var(--fg-secondary)", fontSize: 14, lineHeight: 1.6, maxWidth: 560 }}>
            Deletion removes your photos, captions, follows, and account data. EXIF and image files are erased.
            Comments you've made on others' frames remain attributed to "[deleted]".
          </p>
          <button className="btn btn-danger" style={{ marginTop: 16, borderColor: "var(--danger)", color: "var(--danger)" }}>
            Delete account…
          </button>
        </div>
      </P8Section>
    </SettingsShell>
  );
};

/* ============================================================
   1B. SETTINGS — alt: tabbed top-level
   ============================================================ */
window.ScreenSettingsTabbed = function ScreenSettingsTabbed({ marks }) {
  const AppHeader = window.AppHeader;
  return (
    <div className="screen" style={{ width: "1440px", overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <section style={{ padding: "40px 64px 0" }}>
        <div className="t-eyebrow">PREFERENCES</div>
        <h1 style={{ fontFamily: "var(--font-display)", fontSize: 48, fontWeight: 400, margin: "8px 0 24px" }}>
          Account <em>settings</em>
        </h1>
        <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-subtle)" }}>
          {["PROFILE", "EQUIPMENT", "NOTIFICATIONS", "EMAIL & SECURITY", "APPEARANCE", "SESSIONS"].map((t, i) => (
            <a key={t} className={"nav-link" + (i === 3 ? " active" : "")} style={{ padding: "16px 20px" }}>{t}</a>
          ))}
          <div style={{ marginLeft: "auto", padding: "16px 0" }}>
            <a className="nav-link" style={{ color: "var(--danger)" }}>DELETE ACCOUNT</a>
          </div>
        </div>
      </section>
      <section style={{ padding: "40px 64px", maxWidth: 1000 }}>
        <div className="t-eyebrow" style={{ marginBottom: 8, color: "var(--accent)" }}>● TAB · 04 OF 06</div>
        <h2 style={{ fontFamily: "var(--font-display)", fontSize: 32, fontWeight: 400, margin: "0 0 32px" }}>
          Email & <em>security</em>
        </h2>
        <P8Section title="Sign-in identity" desc="Your email is how you sign in.">
          <P8Row label="EMAIL"><div style={{display:"flex", gap:8}}><input className="input" defaultValue="marie.dubois@example.fr" style={{flex:1}}/><button className="btn btn-secondary">Change…</button></div></P8Row>
          <P8Row label="PASSWORD"><div style={{display:"flex", gap:8}}><input className="input" type="password" defaultValue="••••••••••" style={{flex:1}} disabled/><button className="btn btn-secondary">Change…</button></div></P8Row>
        </P8Section>
        <P8Section title="Two-factor authentication" desc="2FA is currently on. Authy · added 02 Feb 2026.">
          <div style={{display:"flex",gap:12}}>
            <button className="btn btn-secondary">View backup codes</button>
            <button className="btn btn-danger">Disable 2FA</button>
          </div>
        </P8Section>
      </section>
    </div>
  );
};

/* ============================================================
   2. PASSWORD RESET — three-step flow + in-settings change
   ============================================================ */
window.ScreenResetRequest = function ScreenResetRequest({ marks }) {
  const Wordmark = marks.Wordmark;
  return (
    <div className="screen" style={{ width: "720px", height: "900px", padding: "64px", display: "flex", flexDirection: "column", justifyContent: "center" }}>
      <Wordmark size={28} italic={true}>Astrophoto</Wordmark>
      <div className="t-eyebrow" style={{ marginTop: 48, marginBottom: 16 }}>RESET PASSWORD · 01 OF 03</div>
      <h1 style={{ fontFamily: "var(--font-display)", fontSize: 40, fontWeight: 400, margin: 0, lineHeight: 1.05 }}>
        We'll send you a link<br/>to <em>find your way back</em>.
      </h1>
      <p style={{ marginTop: 16, color: "var(--fg-secondary)", fontSize: 14, maxWidth: 480 }}>
        Enter the email associated with your account. The link is single-use and expires in one hour.
      </p>
      <div style={{ marginTop: 32, maxWidth: 480, display: "flex", flexDirection: "column", gap: 16 }}>
        <div>
          <div className="t-label" style={{ marginBottom: 6 }}>EMAIL</div>
          <input className="input" placeholder="you@somewhere.com"/>
        </div>
        <button className="btn btn-primary btn-lg">Send reset link</button>
        <a className="t-meta" style={{ color: "var(--accent)", marginTop: 8 }}>← Back to sign in</a>
      </div>
    </div>
  );
};

window.ScreenResetSent = function ScreenResetSent({ marks }) {
  const Wordmark = marks.Wordmark;
  const Mark = marks.MarkReticle;
  return (
    <div className="screen" style={{ width: "720px", height: "900px", padding: "64px", display: "flex", flexDirection: "column", justifyContent: "center", textAlign: "center" }}>
      <div style={{ alignSelf: "center" }}><Mark size={56} color="var(--accent)"/></div>
      <div className="t-eyebrow" style={{ marginTop: 32, marginBottom: 16, color: "var(--accent)" }}>● RESET PASSWORD · 02 OF 03 · CHECK YOUR EMAIL</div>
      <h1 style={{ fontFamily: "var(--font-display)", fontSize: 40, fontWeight: 400, margin: 0, lineHeight: 1.05 }}>
        A link is on its way<br/>to <em>marie.dubois@example.fr</em>
      </h1>
      <p style={{ marginTop: 16, color: "var(--fg-secondary)", fontSize: 14, maxWidth: 460, marginLeft: "auto", marginRight: "auto" }}>
        It should arrive within a minute. The link works once and expires in one hour.
        Look in spam if it doesn't appear — sometimes it goes there.
      </p>
      <div style={{ marginTop: 40, padding: 24, border: "1px dashed var(--border-default)", maxWidth: 520, marginLeft: "auto", marginRight: "auto", textAlign: "left" }}>
        <div className="t-label" style={{ color: "var(--fg-muted)" }}>EMAIL PREVIEW · PLAIN TEXT</div>
        <pre style={{
          fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--fg-secondary)",
          whiteSpace: "pre-wrap", margin: "16px 0 0", lineHeight: 1.7,
        }}>{`From:    Astrophoto <mail@astrophoto.pics>
To:      marie.dubois@example.fr
Subject: Reset your Astrophoto password

Hello Marie,

Someone — we hope it was you — asked to reset
your Astrophoto password. Open this link to
choose a new one:

  https://astrophoto.pics/reset/4f8a-2c1d-9b7e

The link works once and expires in one hour.
If it wasn't you, ignore this message; nothing
will change.

Clear skies,
The Astrophoto archive
52°31′N · 13°24′E`}</pre>
      </div>
      <div style={{ marginTop: 32, display: "flex", gap: 12, justifyContent: "center" }}>
        <button className="btn btn-ghost">Resend in 0:42</button>
        <button className="btn btn-secondary">Use a different email</button>
      </div>
    </div>
  );
};

window.ScreenResetSetNew = function ScreenResetSetNew({ marks }) {
  const Wordmark = marks.Wordmark;
  return (
    <div className="screen" style={{ width: "720px", height: "900px", padding: "64px", display: "flex", flexDirection: "column", justifyContent: "center" }}>
      <Wordmark size={28} italic={true}>Astrophoto</Wordmark>
      <div className="t-eyebrow" style={{ marginTop: 48, marginBottom: 16, color: "var(--accent)" }}>RESET PASSWORD · 03 OF 03</div>
      <h1 style={{ fontFamily: "var(--font-display)", fontSize: 40, fontWeight: 400, margin: 0, lineHeight: 1.05 }}>
        Choose a <em>new password</em>.
      </h1>
      <p style={{ marginTop: 16, color: "var(--fg-secondary)", fontSize: 14, maxWidth: 480 }}>
        Resetting for <strong style={{ color: "var(--fg-primary)" }}>marie.dubois@example.fr</strong>.
        At least 10 characters, including one number or symbol.
      </p>
      <div style={{ marginTop: 32, maxWidth: 480, display: "flex", flexDirection: "column", gap: 16 }}>
        <div>
          <div className="t-label" style={{ marginBottom: 6 }}>NEW PASSWORD</div>
          <input className="input" type="password" defaultValue="••••••••••••"/>
          {/* strength meter */}
          <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
            {[1,2,3,4].map(i => (
              <div key={i} style={{ flex: 1, height: 3, background: i <= 3 ? "var(--accent)" : "var(--border-default)" }}/>
            ))}
          </div>
          <div className="t-meta" style={{ marginTop: 6, color: "var(--accent)" }}>STRONG · ESTIMATED 200 YEARS TO CRACK</div>
        </div>
        <div>
          <div className="t-label" style={{ marginBottom: 6 }}>CONFIRM NEW PASSWORD</div>
          <input className="input" type="password" defaultValue="••••••••••••"/>
        </div>
        <div style={{ padding: 12, background: "var(--bg-warning-tint)", border: "1px solid var(--warning)", borderRadius: 2, fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--fg-secondary)", letterSpacing: "0.04em" }}>
          ⚠ ALL OTHER SESSIONS WILL BE SIGNED OUT WHEN YOU SAVE.
        </div>
        <button className="btn btn-primary btn-lg" style={{ marginTop: 8 }}>Set new password & sign in</button>
      </div>
    </div>
  );
};

window.ScreenChangePassword = function ScreenChangePassword({ marks }) {
  // In-settings dialog rendered inline at full artboard size
  return (
    <div className="screen" style={{ width: "640px", height: "720px", padding: "48px", background: "var(--bg-raised)", border: "1px solid var(--border-default)" }}>
      <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● CHANGE PASSWORD · IN-SETTINGS</div>
      <h2 style={{ fontFamily: "var(--font-display)", fontSize: 32, fontWeight: 400, margin: "8px 0 0", fontStyle: "italic" }}>
        Change password
      </h2>
      <p style={{ color: "var(--fg-secondary)", fontSize: 13, marginTop: 12, maxWidth: 480 }}>
        Confirm with your current password to make this change. We sign you out of every other device on save.
      </p>
      <div style={{ marginTop: 24, display: "flex", flexDirection: "column", gap: 16 }}>
        <div>
          <div className="t-label" style={{ marginBottom: 6 }}>CURRENT PASSWORD</div>
          <input className="input" type="password" defaultValue="••••••••••"/>
          <a className="t-meta" style={{ color: "var(--accent)", marginTop: 6, display: "inline-block" }}>I don't remember it →</a>
        </div>
        <div>
          <div className="t-label" style={{ marginBottom: 6 }}>NEW PASSWORD</div>
          <input className="input" type="password" defaultValue="••••••••••••"/>
          <div style={{ display: "flex", gap: 4, marginTop: 8 }}>
            {[1,2,3,4].map(i => (
              <div key={i} style={{ flex: 1, height: 3, background: i <= 3 ? "var(--accent)" : "var(--border-default)" }}/>
            ))}
          </div>
        </div>
        <div>
          <div className="t-label" style={{ marginBottom: 6 }}>CONFIRM NEW PASSWORD</div>
          <input className="input" type="password" defaultValue="••••••••••••"/>
        </div>
      </div>
      <div style={{ marginTop: 32, display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn btn-ghost">Cancel</button>
        <button className="btn btn-primary">Save new password</button>
      </div>
    </div>
  );
};

/* ============================================================
   3. 2FA SETUP + BACKUP CODES
   ============================================================ */
window.Screen2FASetup = function Screen2FASetup({ marks }) {
  // Fake QR pattern
  const QR_BITS = [
    "1111111010101011111111","1000001011001011000001","1011101001110011011101","1011101010100011011101",
    "1011101001011011011101","1000001010100111000001","1111111010101011111111","0000000011110100000000",
    "1010110100100110011101","0110001110001011010110","1011010101110100110010","1100100101110001100110",
    "0010111010001000110001","1001100100110111001011","1010010011011011000110","0101101001110100110011",
    "0000000010110011010101","1111111010100010111101","1000001011010110011001","1011101001110100110011",
    "1011101011000010101110","1000001010110011110001",
  ];
  return (
    <div className="screen" style={{ width: "1080px", height: "880px", padding: "64px", background: "var(--bg-raised)", border: "1px solid var(--border-default)" }}>
      <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● TWO-FACTOR · 01 OF 02 · SCAN OR PASTE</div>
      <h2 style={{ fontFamily: "var(--font-display)", fontSize: 36, fontWeight: 400, margin: "8px 0 0" }}>
        Set up <em>two-factor authentication</em>
      </h2>
      <p style={{ color: "var(--fg-secondary)", fontSize: 14, marginTop: 12, maxWidth: 640 }}>
        Open an authenticator app — Authy, 1Password, or your phone's built-in — and scan this code.
        It will produce a six-digit number that changes every 30 seconds.
      </p>
      <div style={{ marginTop: 32, display: "grid", gridTemplateColumns: "320px 1fr", gap: 48 }}>
        <div>
          <div style={{ background: "#fff", padding: 16, width: 264, height: 264, position: "relative" }}>
            <svg viewBox="0 0 22 22" width="232" height="232" shapeRendering="crispEdges">
              {QR_BITS.map((row, y) => row.split("").map((b, x) =>
                b === "1" ? <rect key={`${x}-${y}`} x={x} y={y} width="1" height="1" fill="#0c0a08"/> : null
              ))}
            </svg>
          </div>
          <div className="t-meta" style={{ marginTop: 12 }}>OR PASTE THIS SECRET INTO THE APP</div>
          <div style={{
            marginTop: 8, padding: "10px 12px", background: "var(--bg-base)",
            border: "1px solid var(--border-default)", borderRadius: 2,
            fontFamily: "var(--font-mono)", fontSize: 13, letterSpacing: "0.08em",
            display: "flex", justifyContent: "space-between", alignItems: "center",
          }}>
            <span>JBSW Y3DP EHPK 3PXP</span>
            <a className="t-meta" style={{ color: "var(--accent)" }}>Copy</a>
          </div>
        </div>
        <div>
          <div className="t-label" style={{ color: "var(--fg-muted)" }}>STEP 02 — ENTER THE 6-DIGIT CODE</div>
          <p style={{ color: "var(--fg-secondary)", fontSize: 13, marginTop: 8, marginBottom: 16 }}>
            From your authenticator app, to confirm it's set up correctly.
          </p>
          <div style={{ display: "flex", gap: 8 }}>
            {["4","2","8","9","1","6"].map((d, i) => (
              <input key={i} className="input input-mono" defaultValue={d} maxLength={1} style={{
                width: 56, height: 64, textAlign: "center", fontSize: 24, padding: 0,
                borderColor: "var(--accent)",
              }}/>
            ))}
          </div>
          <div className="t-meta" style={{ marginTop: 12, color: "var(--success)" }}>● CODE VERIFIED · CONTINUING IN 1S…</div>

          <div style={{ marginTop: 40, padding: 16, border: "1px dashed var(--border-default)" }}>
            <div className="t-label" style={{ color: "var(--accent)" }}>WHAT IF I LOSE MY PHONE?</div>
            <p style={{ margin: "8px 0 0", fontSize: 12, color: "var(--fg-secondary)", lineHeight: 1.6 }}>
              On the next screen we give you ten one-time backup codes.
              Save them somewhere offline. Each one signs you in once, even without the app.
            </p>
          </div>
        </div>
      </div>
      <div style={{ marginTop: 40, display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn btn-ghost">Cancel</button>
        <button className="btn btn-primary">Continue → backup codes</button>
      </div>
    </div>
  );
};

window.Screen2FABackup = function Screen2FABackup({ marks }) {
  const codes = [
    "8KP2-7HQR-9XLF","2NMJ-4DTB-7GVC","5LWA-1ZRY-6PEK",
    "9XCF-3HJM-2BNQ","7GTD-4VPL-1KMR","3RBN-8YHQ-2LFC",
    "6WEK-2JTV-9XPB","1QMR-5HBC-7TGD","4PNL-9KFY-3RVH",
    "8XBT-6CGM-1JEW",
  ];
  return (
    <div className="screen" style={{ width: "1080px", height: "880px", padding: "64px", background: "var(--bg-raised)", border: "1px solid var(--border-default)" }}>
      <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● TWO-FACTOR · 02 OF 02 · BACKUP CODES</div>
      <h2 style={{ fontFamily: "var(--font-display)", fontSize: 36, fontWeight: 400, margin: "8px 0 0" }}>
        Save these <em>somewhere offline</em>.
      </h2>
      <p style={{ color: "var(--fg-secondary)", fontSize: 14, marginTop: 12, maxWidth: 640 }}>
        Ten one-time codes. Each works exactly once and signs you in if you lose your authenticator.
        Print them, save to a password manager, or tape them inside a notebook. We won't show them again.
      </p>
      <div style={{ marginTop: 32, padding: 24, border: "1px solid var(--border-default)", background: "var(--bg-base)" }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: "12px 64px", fontFamily: "var(--font-mono)", fontSize: 16, letterSpacing: "0.06em" }}>
          {codes.map((c, i) => (
            <div key={i} style={{ display: "flex", justifyContent: "space-between", borderBottom: "1px dashed var(--border-subtle)", padding: "8px 0" }}>
              <span style={{ color: "var(--fg-faint)" }}>{String(i + 1).padStart(2, "0")}</span>
              <span style={{ color: "var(--fg-primary)" }}>{c}</span>
            </div>
          ))}
        </div>
      </div>
      <div style={{ marginTop: 24, display: "flex", gap: 8, alignItems: "center" }}>
        <button className="btn btn-secondary">📋 Copy all</button>
        <button className="btn btn-secondary">⤓ Download .txt</button>
        <button className="btn btn-secondary">🖨 Print</button>
        <span className="t-meta" style={{ marginLeft: "auto", color: "var(--warning)" }}>● THESE WILL NOT BE SHOWN AGAIN</span>
      </div>
      <div style={{ marginTop: 32, display: "flex", gap: 12, alignItems: "center", justifyContent: "flex-end" }}>
        <label style={{ display: "flex", gap: 8, alignItems: "center", fontSize: 13, color: "var(--fg-secondary)" }}>
          <input type="checkbox" defaultChecked/> I've saved my backup codes somewhere safe.
        </label>
        <button className="btn btn-primary">Finish 2FA setup</button>
      </div>
    </div>
  );
};

/* ============================================================
   4. ACCOUNT DELETION CONFIRM (modal)
   ============================================================ */
window.ScreenDeleteConfirm = function ScreenDeleteConfirm({ marks }) {
  return (
    <div className="screen" style={{ width: "640px", height: "780px", padding: "48px", background: "var(--bg-raised)", border: "1px solid var(--danger)" }}>
      <div className="t-eyebrow" style={{ color: "var(--danger)" }}>● DELETE ACCOUNT · CONFIRMATION</div>
      <h2 style={{ fontFamily: "var(--font-display)", fontSize: 32, fontWeight: 400, margin: "8px 0 0", fontStyle: "italic", color: "var(--danger)" }}>
        Are you sure?
      </h2>
      <p style={{ color: "var(--fg-secondary)", fontSize: 14, marginTop: 16, lineHeight: 1.65 }}>
        Deletion begins a <strong style={{ color: "var(--fg-primary)" }}>7-day grace period</strong>.
        During that week your account is hidden from the public archive but you can sign in to cancel.
        After 7 days everything below is permanently erased.
      </p>
      <div style={{ marginTop: 24, padding: 20, background: "var(--bg-danger-tint)", border: "1px solid var(--border-default)" }}>
        <div className="t-label" style={{ color: "var(--fg-muted)", marginBottom: 12 }}>WHAT WILL BE ERASED</div>
        <ul style={{ margin: 0, paddingLeft: 20, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--fg-secondary)", lineHeight: 2 }}>
          <li>42 published frames + 3 drafts (318 hours of integration)</li>
          <li>14,206 appreciations received · 3,082 comments</li>
          <li>EXIF, plate-solves, equipment profile</li>
          <li>Your handle <span style={{ color: "var(--fg-primary)" }}>@marie.dubois</span> (releasable after 90 days)</li>
        </ul>
        <div className="t-label" style={{ color: "var(--fg-muted)", margin: "16px 0 8px" }}>WHAT REMAINS</div>
        <p style={{ margin: 0, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--fg-muted)", lineHeight: 1.7 }}>
          Comments you've made on others' frames stay attributed to "[deleted]".
        </p>
      </div>
      <div style={{ marginTop: 24 }}>
        <div className="t-label" style={{ marginBottom: 6 }}>TYPE <span style={{ color: "var(--danger)", letterSpacing: "0.16em" }}>DELETE MY ACCOUNT</span> TO CONFIRM</div>
        <input className="input input-mono" placeholder="DELETE MY ACCOUNT"/>
      </div>
      <div style={{ marginTop: 24, display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="btn btn-secondary">Keep my account</button>
        <button className="btn btn-danger" style={{ color: "var(--danger)", borderColor: "var(--danger)" }}>
          Begin 7-day deletion
        </button>
      </div>
    </div>
  );
};

Object.assign(window, {
  SettingsShell, P8Section, P8Row,
});
