/* =============================================================
   ASTROPHOTO · SHOWCASE PHASE 1 — Foundations
   Multi-file upload (dropzone + file rows + verify), handle picker.
   ============================================================= */

const PHOTOS_S = window.PHOTOS;

/* ---------- Helpers ---------- */
const Eyebrow = ({ children, accent, danger, warning }) => (
  <div className="t-eyebrow" style={{
    color: danger ? "var(--danger)" : warning ? "var(--warning)" : accent ? "var(--accent)" : undefined
  }}>{children}</div>
);

const Display = ({ children, size = 48, style }) => (
  <h1 style={{
    fontFamily: "var(--font-display)", fontSize: size, fontWeight: 400,
    margin: "8px 0 0", lineHeight: 1.02, letterSpacing: "-0.015em", ...style
  }}>{children}</h1>
);

const PageHeader = ({ eyebrow, title, right, children }) => (
  <section style={{
    padding: "40px 64px 24px",
    borderBottom: "1px solid var(--border-subtle)",
    display: "flex", justifyContent: "space-between", alignItems: "end", gap: 32,
  }}>
    <div>
      <Eyebrow>{eyebrow}</Eyebrow>
      <Display>{title}</Display>
      {children}
    </div>
    {right}
  </section>
);

const Stepper = ({ steps }) => (
  <div style={{
    display: "flex", gap: 0, marginTop: 32,
    fontFamily: "var(--font-mono)", fontSize: 11,
    letterSpacing: "0.12em", textTransform: "uppercase",
  }}>
    {steps.map(([n, l, s]) => (
      <div key={n} style={{
        flex: 1, padding: "16px 0",
        borderTop: `2px solid ${s ? "var(--accent)" : "var(--border-default)"}`,
        color: s ? "var(--fg-primary)" : "var(--fg-muted)",
        display: "flex", gap: 12, alignItems: "center",
      }}>
        <span style={{ color: s ? "var(--accent)" : "var(--fg-faint)" }}>{n}</span>
        <span>{l}</span>
        {s === "done" && <span style={{ color: "var(--accent)", marginLeft: "auto", marginRight: 32 }}>✓</span>}
      </div>
    ))}
  </div>
);

window.AP_Eyebrow = Eyebrow;
window.AP_Display = Display;
window.AP_PageHeader = PageHeader;
window.AP_Stepper = Stepper;

/* ============================================================
   1A — UPLOAD · DROPZONE & MULTI-FILE QUEUE
   ============================================================ */
window.ScreenUploadDropzone = function ({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  const photos = PHOTOS_S;
  return (
    <div className="screen" style={{ width: 1440, height: 1500, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <PageHeader
        eyebrow="NEW UPLOAD · STEP 01 OF 03"
        title={<>Drop your <em>frames</em></>}
        right={
          <div style={{ textAlign: "right" }}>
            <div className="t-eyebrow" style={{ color: "var(--accent)" }}>● FREE TIER · 50 MB / FILE</div>
            <p style={{ margin: "8px 0 0", color: "var(--fg-muted)", fontSize: 12, maxWidth: 280 }}>
              Subscribers upload up to 200 MB.{" "}
              <a style={{ color: "var(--accent)", textDecoration: "underline" }}>Upgrade →</a>
            </p>
          </div>
        }
      >
        <Stepper steps={[["01", "UPLOAD", "active"], ["02", "VERIFY EACH", ""], ["03", "PUBLISH", ""]]}/>
      </PageHeader>

      <section style={{ padding: "32px 64px 0" }}>
        {/* Dropzone */}
        <div style={{
          border: "1px dashed var(--accent)",
          background: "var(--bg-accent-tint)",
          padding: "56px 32px",
          textAlign: "center",
          position: "relative",
        }}>
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none"
               stroke="var(--accent)" strokeWidth="1.2"
               style={{ margin: "0 auto 16px", display: "block" }}>
            <path d="M12 16V4M6 10l6-6 6 6M4 20h16"/>
          </svg>
          <div style={{
            fontFamily: "var(--font-display)", fontSize: 28, fontStyle: "italic",
            color: "var(--accent)", marginBottom: 8,
          }}>Drop photos here, or click to browse</div>
          <p style={{ color: "var(--fg-secondary)", fontSize: 13, margin: 0 }}>
            JPEG · PNG · TIFF (16-bit) &nbsp;·&nbsp; up to 50 MB per file &nbsp;·&nbsp; up to 12 at once
          </p>
          <div style={{
            position: "absolute", top: 12, right: 12,
            display: "flex", gap: 6, alignItems: "center",
          }}>
            <div className="chip" style={{ borderColor: "var(--accent-dim)" }}>⌘ + V to paste from clipboard</div>
          </div>
        </div>

        {/* Queue header */}
        <div style={{
          display: "flex", justifyContent: "space-between", alignItems: "center",
          margin: "32px 0 12px",
        }}>
          <Eyebrow>FILES · 4 · 2 ready · 1 uploading · 1 blocked</Eyebrow>
          <div style={{ display: "flex", gap: 8 }}>
            <button className="chip">Clear queue</button>
            <button className="btn btn-secondary btn-sm">Add more files</button>
          </div>
        </div>

        {/* File rows */}
        <div style={{ border: "1px solid var(--border-subtle)" }}>
          <FileRow photo={photos[7]} state="ready" name="NGC7000_SHO_v3.tif"
                   size="48.2 MB" pct={100} hash="sha256:b21f…ef0c"
                   detail="Saved as draft · target M31 · 4 frames hashed"/>
          <FileRow photo={photos[1]} state="uploading" name="m31_lrgb_master.jpg"
                   size="12.4 MB" pct={62}
                   detail="Streaming PUT to S3 · 7.7 / 12.4 MB · 2.1 MB/s"/>
          <FileRow photo={photos[5]} state="hashing" name="m42_orion_2026.tif"
                   size="36.8 MB" pct={18}
                   detail="Computing SHA-256 · 6.7 / 36.8 MB"/>
          <FileRow photo={photos[3]} state="failed" name="ngc6960_h_alpha.tif"
                   size="184 MB" pct={0}
                   detail="Too large for free tier · max 50 MB"
                   action="upgrade"/>
        </div>

        {/* Footer actions */}
        <div style={{
          display: "flex", justifyContent: "space-between", alignItems: "center",
          marginTop: 24, paddingTop: 24, borderTop: "1px dashed var(--border-default)",
        }}>
          <div className="t-meta" style={{ color: "var(--fg-muted)" }}>
            STORAGE · 1.84 / 5.00 GB USED · 36 % &nbsp;·&nbsp; CHECKSUM DEDUP IS PER-OWNER
          </div>
          <div style={{ display: "flex", gap: 12 }}>
            <button className="btn btn-ghost">Save & finish later</button>
            <button className="btn btn-primary">Verify 2 ready frames →</button>
          </div>
        </div>
      </section>
    </div>
  );
};

function FileRow({ photo, state, name, size, pct, hash, detail, action }) {
  const Photo = window.Photo;
  const stateColor = {
    ready: "var(--accent)",
    uploading: "var(--accent)",
    hashing: "var(--info)",
    failed: "var(--danger)",
  }[state];
  const stateLabel = {
    ready: "✓ READY",
    uploading: "↑ UPLOADING",
    hashing: "◐ HASHING",
    failed: "✗ FAILED",
  }[state];
  return (
    <div style={{
      display: "grid", gridTemplateColumns: "72px 1fr 220px 180px 32px",
      gap: 16, padding: "16px 16px",
      borderBottom: "1px dashed var(--border-subtle)",
      alignItems: "center",
      background: state === "failed" ? "var(--bg-danger-tint)" : "transparent",
    }}>
      <div style={{ position: "relative", width: 72, height: 72 }}>
        <Photo photo={photo} style={{ position: "absolute", inset: 0 }}/>
        {state === "failed" && (
          <div style={{
            position: "absolute", inset: 0,
            background: "rgba(168,69,58,.45)", border: "1px solid var(--danger)",
          }}/>
        )}
      </div>
      <div style={{ minWidth: 0 }}>
        <div style={{
          fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--fg-primary)",
          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
        }}>{name}</div>
        <div className="t-meta" style={{ marginTop: 4, color: "var(--fg-muted)" }}>
          {size} {hash && <>· {hash}</>}
        </div>
        <div style={{
          fontFamily: "var(--font-mono)", fontSize: 11, marginTop: 6,
          color: state === "failed" ? "var(--danger)" : "var(--fg-secondary)",
        }}>{detail}</div>
      </div>
      <div>
        {state !== "failed" ? (
          <>
            <div style={{
              height: 4, background: "var(--border-default)",
              position: "relative", overflow: "hidden",
            }}>
              <div style={{
                position: "absolute", inset: 0, width: `${pct}%`,
                background: stateColor,
              }}/>
            </div>
            <div className="t-meta" style={{
              marginTop: 6, display: "flex", justifyContent: "space-between",
              color: stateColor,
            }}>
              <span>{stateLabel}</span>
              <span>{pct}%</span>
            </div>
          </>
        ) : (
          <div className="t-meta" style={{ color: "var(--danger)" }}>
            ● BLOCKED BEFORE BANDWIDTH USED
          </div>
        )}
      </div>
      <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
        {state === "ready" && <button className="btn btn-secondary btn-sm">✏ Edit</button>}
        {state === "uploading" && <button className="btn btn-ghost btn-sm">Pause</button>}
        {state === "hashing" && <button className="btn btn-ghost btn-sm">Cancel</button>}
        {action === "upgrade" && <>
          <button className="btn btn-secondary btn-sm">Replace</button>
          <button className="btn btn-primary btn-sm">Upgrade</button>
        </>}
      </div>
      <div style={{ color: "var(--fg-faint)", textAlign: "right" }}>✕</div>
    </div>
  );
}

/* ============================================================
   1B — UPLOAD · VERIFY (target / tags / category / equipment)
   ============================================================ */
window.ScreenUploadVerify = function ({ marks }) {
  const AppHeader = window.AppHeader;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: 1440, height: 1700, overflow: "hidden" }}>
      <AppHeader auth marks={marks}/>
      <PageHeader
        eyebrow="STEP 02 OF 03 · VERIFY · FRAME 1 OF 2"
        title={<>What's <em>in</em> this frame?</>}
        right={
          <div style={{ textAlign: "right" }}>
            <div className="t-meta" style={{ color: "var(--accent)" }}>● 14 EXIF FIELDS RECOVERED</div>
            <div className="t-meta" style={{ color: "var(--fg-muted)", marginTop: 6 }}>
              CLIENT THUMBNAIL READY · BLURHASH GENERATED
            </div>
          </div>
        }
      >
        <Stepper steps={[["01", "UPLOAD", "done"], ["02", "VERIFY EACH", "active"], ["03", "PUBLISH", ""]]}/>
      </PageHeader>

      <section style={{ padding: "40px 64px", display: "grid", gridTemplateColumns: "520px 1fr", gap: 64 }}>
        {/* Preview */}
        <div>
          <div className="t-label" style={{ marginBottom: 12 }}>PREVIEW · CLIENT BITMAP</div>
          <div style={{ position: "relative", aspectRatio: "4/3" }}>
            <Photo photo={PHOTOS_S[7]} style={{ position: "absolute", inset: 0 }}/>
            <div style={{
              position: "absolute", left: 12, top: 12,
              padding: "4px 8px", background: "rgba(12,10,8,.85)",
              border: "1px solid var(--accent-dim)",
              fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--accent)",
              letterSpacing: "0.08em",
            }}>● UPLOADED · DISPLAY MASTER QUEUED</div>
          </div>
          <div className="t-meta" style={{ marginTop: 12, display: "flex", justifyContent: "space-between" }}>
            <span>NGC7000_SHO_v3.tif</span>
            <span>48.2 MB · 6248 × 4176</span>
          </div>

          {/* Frame nav */}
          <div style={{
            marginTop: 20, padding: 16, border: "1px solid var(--border-default)",
            background: "var(--bg-base)",
          }}>
            <div className="t-label" style={{ marginBottom: 8 }}>QUEUE · FRAMES TO VERIFY</div>
            <div style={{ display: "flex", gap: 8 }}>
              {[0, 1].map(i => (
                <div key={i} style={{
                  position: "relative", width: 80, height: 60,
                  border: i === 0 ? "1px solid var(--accent)" : "1px solid var(--border-default)",
                }}>
                  <Photo photo={PHOTOS_S[i === 0 ? 7 : 1]} style={{ position: "absolute", inset: 0 }}/>
                  {i === 0 && <div style={{
                    position: "absolute", inset: -1, border: "1px solid var(--accent)",
                    pointerEvents: "none",
                  }}/>}
                </div>
              ))}
            </div>
            <div className="t-meta" style={{ marginTop: 10, color: "var(--fg-muted)" }}>
              ← / → BETWEEN FRAMES · ⌘↩ TO PUBLISH ALL
            </div>
          </div>
        </div>

        {/* Form */}
        <div>
          {/* Discovery group — NEW */}
          <SectionCard
            title="DISCOVERY"
            sub="These four fields decide where this frame appears across the site."
            accent
          >
            <FieldGroup>
              <Field label="Target" sub="Picker · M, NGC, IC, Caldwell, common names" full>
                <TargetPicker/>
              </Field>
              <Field label="Category" full>
                <CategorySegmented/>
              </Field>
              <Field label="Tags" sub="Up to 8 · free-form · autocomplete" full>
                <TagInput/>
              </Field>
            </FieldGroup>
          </SectionCard>

          {/* Equipment — autocomplete-fed */}
          <SectionCard
            title="EQUIPMENT USED"
            sub="Pre-filled from your default loadout. Each field becomes a browsable equipment page."
          >
            <FieldGroup>
              <Field label="Telescope (scope)" autocomplete prefilled>
                <input className="input input-mono" defaultValue="Takahashi FSQ-106EDX4"/>
              </Field>
              <Field label="Camera" autocomplete fromExif>
                <input className="input input-mono" defaultValue="ZWO ASI2600MC Pro"/>
              </Field>
              <Field label="Mount" autocomplete prefilled>
                <input className="input input-mono" defaultValue="10Micron GM1000 HPS"/>
              </Field>
              <Field label="Filters" autocomplete prefilled>
                <input className="input input-mono" defaultValue="Antlia 3 nm SHO"/>
              </Field>
              <Field label="Guiding" autocomplete prefilled>
                <input className="input input-mono" defaultValue="ASI120MM Mini · OAG"/>
              </Field>
            </FieldGroup>
          </SectionCard>

          {/* EXIF — recovered */}
          <SectionCard
            title="CAPTURE DATA · FROM EXIF"
            sub="Editable, not required. We never overwrite the original."
          >
            <FieldGroup>
              <Field label="Captured" fromExif>
                <input className="input input-mono" defaultValue="14–17 March 2026"/>
              </Field>
              <Field label="Sessions">
                <input className="input input-mono" defaultValue="4"/>
              </Field>
              <Field label="Exposure" fromExif>
                <input className="input input-mono" defaultValue="180 × 360 s"/>
              </Field>
              <Field label="Gain" fromExif>
                <input className="input input-mono" defaultValue="100"/>
              </Field>
            </FieldGroup>
          </SectionCard>

          {/* Plate-solve forward-compat */}
          <div style={{
            marginTop: 24, padding: 16,
            border: "1px dashed var(--accent-dim)",
            background: "var(--bg-accent-tint)",
            display: "flex", alignItems: "center", justifyContent: "space-between", gap: 16,
          }}>
            <div>
              <div className="t-label" style={{ color: "var(--accent)" }}>● COMING SOON · PLATE SOLVE</div>
              <p style={{ margin: "6px 0 0", fontSize: 12, color: "var(--fg-secondary)" }}>
                When the astrometry phase ships, your frames here will be auto-matched to known targets.
                Targets you pick now stay as <code style={{ color: "var(--accent)" }}>source = "manual"</code>.
              </p>
            </div>
            <button className="btn btn-secondary btn-sm" disabled style={{ opacity: 0.45 }}>Run plate solve</button>
          </div>

          {/* Footer actions */}
          <div style={{ marginTop: 32, display: "flex", gap: 12, justifyContent: "flex-end" }}>
            <button className="btn btn-ghost btn-lg">Save as draft</button>
            <button className="btn btn-secondary btn-lg">Skip frame →</button>
            <button className="btn btn-primary btn-lg">Continue · 1 of 2 →</button>
          </div>
        </div>
      </section>
    </div>
  );
};

function SectionCard({ title, sub, accent, children }) {
  return (
    <div style={{
      marginBottom: 28,
      padding: 0,
      borderTop: `1px solid ${accent ? "var(--accent-dim)" : "var(--border-subtle)"}`,
    }}>
      <div style={{ padding: "16px 0 4px" }}>
        <div className="t-label" style={{ color: accent ? "var(--accent)" : "var(--fg-muted)" }}>
          {accent && "● "}{title}
        </div>
        {sub && <p style={{
          margin: "6px 0 16px", fontSize: 12, color: "var(--fg-muted)", maxWidth: 560,
        }}>{sub}</p>}
      </div>
      {children}
    </div>
  );
}
function FieldGroup({ children }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>{children}</div>
  );
}
function Field({ label, sub, full, autocomplete, fromExif, prefilled, children }) {
  return (
    <div style={{ gridColumn: full ? "1 / -1" : "auto" }}>
      <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: 6 }}>
        <span className="t-label">{label}</span>
        <span className="t-meta" style={{ color: fromExif ? "var(--accent)" : prefilled ? "var(--info)" : autocomplete ? "var(--fg-muted)" : "transparent" }}>
          {fromExif ? "from EXIF" : prefilled ? "from your defaults" : autocomplete ? "↘ autocomplete" : ""}
        </span>
      </div>
      {children}
      {sub && <div className="t-meta" style={{ marginTop: 6, color: "var(--fg-faint)" }}>{sub.toUpperCase()}</div>}
    </div>
  );
}

function TargetPicker() {
  return (
    <div style={{ position: "relative" }}>
      <input className="input" defaultValue="m31"/>
      <div style={{
        position: "absolute", left: 0, right: 0, top: 40,
        background: "var(--bg-elevated)", border: "1px solid var(--border-default)",
        boxShadow: "var(--shadow-md)", zIndex: 5,
      }}>
        {[
          ["M31", "Andromeda Galaxy", "messier · NGC 224", true],
          ["M32", "Le Gentil", "messier · companion of M31", false],
          ["NGC 891", "Silver Sliver", "ngc · edge-on galaxy", false],
        ].map(([slug, name, kind, sel], i) => (
          <div key={i} style={{
            padding: "10px 12px",
            background: sel ? "var(--bg-accent-tint)" : "transparent",
            borderLeft: sel ? "2px solid var(--accent)" : "2px solid transparent",
            display: "flex", alignItems: "center", gap: 12,
            borderBottom: "1px dashed var(--border-subtle)",
          }}>
            <span style={{
              fontFamily: "var(--font-mono)", fontSize: 12,
              color: sel ? "var(--accent)" : "var(--fg-secondary)",
              minWidth: 64,
            }}>{slug}</span>
            <span style={{
              fontFamily: "var(--font-display)", fontSize: 15, fontStyle: "italic",
              color: "var(--fg-primary)", flex: 1,
            }}>{name}</span>
            <span className="t-meta" style={{ color: "var(--fg-muted)" }}>{kind.toUpperCase()}</span>
          </div>
        ))}
        <div style={{ padding: "10px 12px", display: "flex", justifyContent: "space-between" }}>
          <span className="t-meta" style={{ color: "var(--fg-muted)" }}>3 OF 482 SHOWN</span>
          <span className="t-meta" style={{ color: "var(--accent)" }}>+ Add free-text target</span>
        </div>
      </div>
    </div>
  );
}

function CategorySegmented() {
  const cats = ["DSO", "Planetary", "Lunar", "Solar", "Wide-field", "Nightscape", "Other"];
  return (
    <div style={{ display: "flex", border: "1px solid var(--border-default)" }}>
      {cats.map((c, i) => (
        <div key={c} style={{
          flex: 1, padding: "8px 0", textAlign: "center",
          fontFamily: "var(--font-mono)", fontSize: 11,
          letterSpacing: "0.08em", textTransform: "uppercase",
          background: i === 0 ? "var(--accent)" : "transparent",
          color: i === 0 ? "var(--accent-ink)" : "var(--fg-secondary)",
          borderRight: i < cats.length - 1 ? "1px solid var(--border-default)" : 0,
          cursor: "pointer",
        }}>{c}</div>
      ))}
    </div>
  );
}

function TagInput() {
  return (
    <div style={{
      minHeight: 36, padding: "6px 8px",
      border: "1px solid var(--border-default)", background: "var(--bg-base)",
      display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center",
    }}>
      {[
        ["narrowband", true],
        ["sho", true],
        ["bicolor", true],
        ["hubble-palette", true],
        ["summer-milky-way", false],
      ].map(([t, set], i) => (
        <span key={i} className="chip" style={set ? {
          borderColor: "var(--accent-dim)", color: "var(--accent)",
          background: "rgba(232,164,58,.06)",
        } : {}}>{t} {set && <span style={{ marginLeft: 4, opacity: 0.6 }}>✕</span>}</span>
      ))}
      <input style={{
        flex: 1, minWidth: 120, height: 22,
        background: "transparent", border: 0, outline: "none",
        color: "var(--fg-primary)", fontFamily: "var(--font-ui)", fontSize: 13,
      }} placeholder="add tag…"/>
      <span className="t-meta" style={{ color: "var(--fg-faint)", marginLeft: "auto" }}>4 / 8</span>
    </div>
  );
}

window.AP_TargetPicker = TargetPicker;
window.AP_TagInput = TagInput;
window.AP_CategorySegmented = CategorySegmented;

/* ============================================================
   1C — HANDLE PICKER (signup) + change in settings
   ============================================================ */
window.ScreenHandlePicker = function ({ marks }) {
  const Wordmark = marks.Wordmark;
  const Mark = marks.MarkReticle;
  const Photo = window.Photo;
  return (
    <div className="screen" style={{ width: 1440, height: 1100, display: "grid", gridTemplateColumns: "1fr 1fr" }}>
      {/* Left — image */}
      <div style={{ position: "relative", overflow: "hidden", background: "#000" }}>
        <Photo photo={PHOTOS_S[1]} style={{ position: "absolute", inset: 0 }}/>
        <div style={{ position: "absolute", inset: 0, background: "linear-gradient(to right, rgba(12,10,8,.82), rgba(12,10,8,.2))" }}/>
        <div style={{ position: "absolute", left: 64, top: 64, display: "flex", alignItems: "center", gap: 14 }}>
          <Mark size={28} color="var(--accent)"/>
          <Wordmark size={24} italic={true}>Astrophoto</Wordmark>
        </div>
        <div style={{ position: "absolute", left: 64, bottom: 64, maxWidth: 520 }}>
          <div className="t-eyebrow" style={{ color: "var(--accent)", marginBottom: 16 }}>● STEP 02 OF 03 · CHOOSE A HANDLE</div>
          <h2 style={{
            fontFamily: "var(--font-display)", fontSize: 40, fontWeight: 400,
            fontStyle: "italic", lineHeight: 1.15, margin: 0,
          }}>
            Your handle is the address of your archive — <span style={{ color: "var(--accent)" }}>astrophoto.pics/u/&lt;your-handle&gt;</span>
          </h2>
          <p style={{ marginTop: 24, color: "var(--fg-secondary)", fontSize: 14, lineHeight: 1.7 }}>
            You can change it later. The old one redirects for 90 days, so links you've shared keep working.
          </p>
        </div>
      </div>

      {/* Right — picker */}
      <div style={{ padding: "120px 96px 0", maxWidth: 600 }}>
        <Eyebrow>SET UP YOUR PROFILE · 02/03</Eyebrow>
        <Display size={44}>Pick a <em>handle</em></Display>
        <p style={{ color: "var(--fg-secondary)", fontSize: 14, marginTop: 16, lineHeight: 1.6 }}>
          3–30 characters. Lowercase letters, numbers, <code className="t-mono" style={{ color: "var(--accent)" }}>-</code> or <code className="t-mono" style={{ color: "var(--accent)" }}>_</code>.
        </p>

        <div style={{ marginTop: 32 }}>
          <HandleField state="available" value="marie-dubois"/>
          <HandleField state="taken" value="andromeda"/>
          <HandleField state="invalid" value="Marie Dubois!"/>
          <HandleField state="reserved" value="admin"/>
          <HandleField state="checking" value="rho-ophiuchi"/>
        </div>

        <div style={{
          marginTop: 32, padding: 16,
          background: "var(--bg-base)", border: "1px dashed var(--border-default)",
          fontSize: 13, color: "var(--fg-secondary)", lineHeight: 1.6,
        }}>
          <div className="t-label" style={{ color: "var(--accent)", marginBottom: 6 }}>YOUR PHOTO PERMALINKS</div>
          Each frame you publish gets an 8-character ID under your handle, e.g.
          <div className="t-mono" style={{
            marginTop: 8, fontSize: 12, color: "var(--fg-primary)",
            padding: "8px 12px", background: "var(--bg-canvas)", border: "1px solid var(--border-subtle)",
          }}>
            astrophoto.pics<span style={{ color: "var(--accent)" }}>/u/marie-dubois</span>/p/<span style={{ color: "var(--accent)" }}>k7Qb9w2x</span>
          </div>
        </div>

        <div style={{ marginTop: 40, display: "flex", gap: 12, justifyContent: "flex-end" }}>
          <button className="btn btn-ghost btn-lg">← Back</button>
          <button className="btn btn-primary btn-lg">Claim handle →</button>
        </div>
      </div>
    </div>
  );
};

function HandleField({ state, value }) {
  const cfg = {
    available: { color: "var(--success)", icon: "✓", msg: "Available." },
    taken:     { color: "var(--danger)",  icon: "✗", msg: "Already taken." },
    invalid:   { color: "var(--danger)",  icon: "✗", msg: "Use 3–30 lowercase letters, numbers, - or _." },
    reserved:  { color: "var(--warning)", icon: "⚠", msg: "Reserved — please choose another." },
    checking:  { color: "var(--info)",    icon: "◐", msg: "Checking…" },
  }[state];
  return (
    <div style={{ marginBottom: 16 }}>
      <div style={{
        display: "flex", alignItems: "stretch",
        border: `1px solid ${state === "available" ? "var(--success)" : state === "taken" || state === "invalid" ? "var(--danger)" : state === "reserved" ? "var(--warning)" : "var(--border-default)"}`,
        background: "var(--bg-base)",
      }}>
        <span style={{
          padding: "0 12px", display: "flex", alignItems: "center",
          fontFamily: "var(--font-mono)", fontSize: 13,
          color: "var(--fg-muted)", borderRight: "1px solid var(--border-subtle)",
        }}>astrophoto.pics/u/</span>
        <input style={{
          flex: 1, height: 44, padding: "0 12px",
          background: "transparent", border: 0, outline: "none",
          color: "var(--fg-primary)", fontFamily: "var(--font-mono)", fontSize: 14,
        }} defaultValue={value}/>
        <span style={{
          padding: "0 12px", display: "flex", alignItems: "center", gap: 6,
          fontFamily: "var(--font-mono)", fontSize: 12,
          color: cfg.color, borderLeft: "1px solid var(--border-subtle)",
          minWidth: 100, justifyContent: "flex-end",
        }}>
          <span>{cfg.icon}</span>
          <span style={{ textTransform: "uppercase", letterSpacing: "0.06em" }}>{state}</span>
        </span>
      </div>
      <div className="t-meta" style={{ marginTop: 6, color: cfg.color }}>{cfg.msg}</div>
    </div>
  );
}

window.AP_HandleField = HandleField;
