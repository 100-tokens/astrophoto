/* ===== LOGO EXPLORATIONS for Astrophoto =====
   Constellation-line glyphs paired with a refined transitional serif.
   8 directions across geometry, proportion, and personality.
*/

const Logo = ({ children, style }) => (
  <div className="logo-block" style={style}>{children}</div>
);

// Reusable serif wordmark
const Wordmark = ({ size = 44, italic = false, tracking = 0, weight = 600, color = "var(--fg-primary)", children = "Astrophoto" }) => (
  <span style={{
    fontFamily: "var(--font-display)",
    fontSize: size,
    fontStyle: italic ? "italic" : "normal",
    fontWeight: weight,
    letterSpacing: `${tracking}em`,
    color,
    lineHeight: 1,
  }}>{children}</span>
);

// ---------- Logomarks (SVG) ----------

// 1. Ursa Major — the most recognizable constellation
const MarkUrsa = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.8">
    <circle cx="32" cy="32" r="30" stroke={color} strokeWidth="0.5" opacity="0.4"/>
    <circle cx="32" cy="32" r="24" stroke={color} strokeWidth="0.3" opacity="0.25" strokeDasharray="2 2"/>
    {/* Big Dipper points */}
    <line x1="14" y1="40" x2="22" y2="36" />
    <line x1="22" y1="36" x2="30" y2="34" />
    <line x1="30" y1="34" x2="38" y2="30" />
    <line x1="38" y1="30" x2="44" y2="22" />
    <line x1="44" y1="22" x2="50" y2="20" />
    <line x1="38" y1="30" x2="46" y2="34" />
    {[[14,40,1.6],[22,36,1.2],[30,34,1.0],[38,30,1.8],[44,22,1.4],[50,20,1.1],[46,34,1.0]].map(([x,y,r],i)=>(
      <circle key={i} cx={x} cy={y} r={r} fill={color} stroke="none"/>
    ))}
  </svg>
);

// 2. Orion — three-belt-star simplification
const MarkOrion = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.8">
    <circle cx="32" cy="32" r="30" stroke={color} strokeWidth="0.5" opacity="0.4"/>
    {/* shoulders, belt, feet */}
    <line x1="18" y1="14" x2="46" y2="18" />
    <line x1="18" y1="14" x2="22" y2="32" />
    <line x1="22" y1="32" x2="42" y2="32" />
    <line x1="42" y1="32" x2="46" y2="18" />
    <line x1="22" y1="32" x2="16" y2="50" />
    <line x1="42" y1="32" x2="48" y2="50" />
    {/* belt stars */}
    <line x1="26" y1="32" x2="38" y2="32" stroke={color} strokeWidth="0.4" strokeDasharray="0.8 1.2"/>
    {[[18,14,1.6],[46,18,1.5],[22,32,1.2],[28,32,1.6],[32,32,1.6],[36,32,1.6],[42,32,1.2],[16,50,1.4],[48,50,1.0]].map(([x,y,r],i)=>(
      <circle key={i} cx={x} cy={y} r={r} fill={color} stroke="none"/>
    ))}
  </svg>
);

// 3. Reticle — center cross + ring (symmetric, mark-like)
const MarkReticle = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="1.4" strokeLinecap="square">
    <circle cx="32" cy="32" r="22" strokeWidth="1.6"/>
    <circle cx="32" cy="32" r="14" strokeDasharray="2.5 2.5" strokeWidth="1" opacity="0.7"/>
    <line x1="32" y1="2" x2="32" y2="18" strokeWidth="1.6"/>
    <line x1="32" y1="46" x2="32" y2="62" strokeWidth="1.6"/>
    <line x1="2" y1="32" x2="18" y2="32" strokeWidth="1.6"/>
    <line x1="46" y1="32" x2="62" y2="32" strokeWidth="1.6"/>
    <circle cx="32" cy="32" r="2.4" fill={color} stroke="none"/>
    {/* small constellation dots inside the ring */}
    <circle cx="24" cy="26" r="1.4" fill={color} stroke="none"/>
    <circle cx="40" cy="36" r="1.7" fill={color} stroke="none"/>
    <circle cx="36" cy="22" r="1.1" fill={color} stroke="none"/>
    <line x1="24" y1="26" x2="36" y2="22" strokeWidth="0.8"/>
    <line x1="36" y1="22" x2="40" y2="36" strokeWidth="0.8"/>
  </svg>
);

// 4. Cassiopeia — the iconic W
const MarkCassiopeia = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.8">
    <circle cx="32" cy="32" r="30" stroke={color} strokeWidth="0.5" opacity="0.35"/>
    <line x1="10" y1="22" x2="22" y2="40"/>
    <line x1="22" y1="40" x2="32" y2="22"/>
    <line x1="32" y1="22" x2="42" y2="40"/>
    <line x1="42" y1="40" x2="54" y2="22"/>
    {[[10,22,1.4],[22,40,1.6],[32,22,1.2],[42,40,1.6],[54,22,1.4]].map(([x,y,r],i)=>(
      <circle key={i} cx={x} cy={y} r={r} fill={color} stroke="none"/>
    ))}
  </svg>
);

// 5. Lyra — small, refined diamond+tail (Vega is the bright star)
const MarkLyra = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.8">
    <circle cx="32" cy="32" r="26" stroke={color} strokeWidth="0.4" opacity="0.3" strokeDasharray="1 2"/>
    <line x1="32" y1="14" x2="44" y2="28"/>
    <line x1="44" y1="28" x2="40" y2="46"/>
    <line x1="40" y1="46" x2="24" y2="46"/>
    <line x1="24" y1="46" x2="20" y2="28"/>
    <line x1="20" y1="28" x2="32" y2="14"/>
    <line x1="32" y1="14" x2="34" y2="6"/>
    {/* Vega — bright, with rays */}
    <circle cx="32" cy="14" r="2.4" fill={color} stroke="none"/>
    <line x1="32" y1="6" x2="32" y2="22" strokeWidth="0.3"/>
    <line x1="24" y1="14" x2="40" y2="14" strokeWidth="0.3"/>
    {[[44,28,1.2],[40,46,1.2],[24,46,1.2],[20,28,1.2],[34,6,0.8]].map(([x,y,r],i)=>(
      <circle key={i} cx={x} cy={y} r={r} fill={color} stroke="none"/>
    ))}
  </svg>
);

// 6. Compass-rose / star atlas plate — for the most "atlas" feel
const MarkAtlas = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.8">
    <circle cx="32" cy="32" r="28"/>
    <circle cx="32" cy="32" r="22" strokeDasharray="1 2" opacity="0.5"/>
    {/* tick marks every 30deg */}
    {Array.from({length:12}).map((_,i)=>{
      const a = (i*30) * Math.PI/180;
      const x1 = 32 + Math.cos(a)*28, y1 = 32 + Math.sin(a)*28;
      const x2 = 32 + Math.cos(a)*(i%3===0?24:26), y2 = 32 + Math.sin(a)*(i%3===0?24:26);
      return <line key={i} x1={x1} y1={y1} x2={x2} y2={y2} strokeWidth={i%3===0?0.8:0.4}/>;
    })}
    {/* north arrow / star */}
    <path d="M 32 12 L 30 24 L 32 22 L 34 24 Z" fill={color} stroke="none"/>
    <text x="32" y="9" textAnchor="middle" fontFamily="var(--font-mono)" fontSize="5" fill={color} stroke="none" letterSpacing="0.1em">N</text>
    {/* one tiny constellation inside */}
    <circle cx="32" cy="32" r="1.2" fill={color}/>
    <circle cx="38" cy="36" r="1" fill={color}/>
    <circle cx="26" cy="38" r="0.8" fill={color}/>
    <line x1="32" y1="32" x2="38" y2="36" strokeWidth="0.4"/>
    <line x1="32" y1="32" x2="26" y2="38" strokeWidth="0.4"/>
  </svg>
);

// 7. Single bright star with diffraction spikes (minimalist)
const MarkStar = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.6">
    <circle cx="32" cy="32" r="3.5" fill={color} stroke="none"/>
    {/* 4 long diffraction spikes */}
    <line x1="32" y1="2" x2="32" y2="62" strokeWidth="0.8"/>
    <line x1="2" y1="32" x2="62" y2="32" strokeWidth="0.8"/>
    {/* 4 short diagonal spikes */}
    <line x1="14" y1="14" x2="50" y2="50" strokeWidth="0.3" opacity="0.5"/>
    <line x1="50" y1="14" x2="14" y2="50" strokeWidth="0.3" opacity="0.5"/>
    {/* halo */}
    <circle cx="32" cy="32" r="9" opacity="0.3"/>
    <circle cx="32" cy="32" r="14" opacity="0.15"/>
  </svg>
);

// 8. Custom monogram — A interlocked with star/asterism
const MarkMonogram = ({ size = 56, color = "currentColor" }) => (
  <svg width={size} height={size} viewBox="0 0 64 64" fill="none" stroke={color} strokeWidth="0.8">
    <rect x="6" y="6" width="52" height="52" stroke={color} strokeWidth="0.5" opacity="0.5"/>
    {/* serif A */}
    <path d="M 14 50 L 26 14 L 38 14 L 50 50" stroke={color} strokeWidth="1.4" fill="none" strokeLinejoin="miter"/>
    <line x1="20" y1="36" x2="44" y2="36" strokeWidth="0.8"/>
    {/* serifs */}
    <line x1="10" y1="50" x2="18" y2="50" strokeWidth="1.2"/>
    <line x1="46" y1="50" x2="54" y2="50" strokeWidth="1.2"/>
    {/* asterism over the A */}
    <circle cx="32" cy="8" r="1" fill={color}/>
    <circle cx="26" cy="6" r="0.6" fill={color}/>
    <circle cx="38" cy="5" r="0.8" fill={color}/>
  </svg>
);

// ---------- LOGO LOCKUPS ----------

const logoFrameStyle = {
  display: "flex",
  flexDirection: "column",
  gap: "20px",
  padding: "40px 36px",
  background: "var(--bg-base)",
  border: "1px solid var(--border-subtle)",
  position: "relative",
  minHeight: "220px",
  justifyContent: "center",
};

const logoFootStyle = {
  position: "absolute",
  bottom: "12px",
  left: "12px",
  right: "12px",
  display: "flex",
  justifyContent: "space-between",
  fontFamily: "var(--font-mono)",
  fontSize: "10px",
  letterSpacing: "0.12em",
  textTransform: "uppercase",
  color: "var(--fg-faint)",
};

const LogoFrame = ({ num, name, mono, children }) => (
  <div style={logoFrameStyle} className="corner-marks">
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", flex: 1, gap: "20px", flexDirection: mono ? "column" : "row" }}>
      {children}
    </div>
    <div style={logoFootStyle}>
      <span>№ {String(num).padStart(2, "0")}</span>
      <span>{name}</span>
    </div>
  </div>
);

// 8 lockups
window.LogoExplorations = [
  // 1 — Ursa Major + serif italic, horizontal
  () => (
    <LogoFrame num={1} name="Ursa · Italic">
      <MarkUrsa size={64} color="var(--accent)"/>
      <Wordmark size={56} italic tracking={0.005}>Astrophoto</Wordmark>
    </LogoFrame>
  ),
  // 2 — Orion stacked (mark above wordmark)
  () => (
    <LogoFrame num={2} name="Orion · Stacked" mono>
      <MarkOrion size={56} color="var(--fg-primary)"/>
      <Wordmark size={48} tracking={0.01}>Astrophoto</Wordmark>
    </LogoFrame>
  ),
  // 3 — Reticle mark, horizontal, accent
  () => (
    <LogoFrame num={3} name="Reticle">
      <MarkReticle size={62} color="var(--accent)"/>
      <Wordmark size={52} weight={500}>Astrophoto</Wordmark>
    </LogoFrame>
  ),
  // 4 — Cassiopeia W under the wordmark like a frontispiece
  () => (
    <LogoFrame num={4} name="Cassiopeia · Plate" mono>
      <Wordmark size={52} italic tracking={0.01}>Astrophoto</Wordmark>
      <MarkCassiopeia size={40} color="var(--accent)"/>
      <div style={{
        fontFamily:"var(--font-mono)",
        fontSize:"10px",
        letterSpacing:"0.32em",
        textTransform:"uppercase",
        color:"var(--fg-muted)"
      }}>EST · MMXXVI · NOCTURNAL</div>
    </LogoFrame>
  ),
  // 5 — Lyra + small caps mono kicker
  () => (
    <LogoFrame num={5} name="Lyra · With kicker" mono>
      <div style={{display:"flex", alignItems:"center", gap:"16px"}}>
        <MarkLyra size={50} color="var(--fg-primary)"/>
        <div style={{display:"flex", flexDirection:"column", alignItems:"flex-start"}}>
          <Wordmark size={46} tracking={0.005}>Astrophoto</Wordmark>
          <div style={{
            fontFamily:"var(--font-mono)",
            fontSize:"10px",
            letterSpacing:"0.24em",
            textTransform:"uppercase",
            color:"var(--accent)",
            marginTop:"4px"
          }}>A QUIET ARCHIVE OF THE NIGHT SKY</div>
        </div>
      </div>
    </LogoFrame>
  ),
  // 6 — Compass / atlas medallion (the "official" feel)
  () => (
    <LogoFrame num={6} name="Atlas · Medallion">
      <MarkAtlas size={70} color="var(--accent)"/>
      <Wordmark size={50} italic>Astrophoto</Wordmark>
    </LogoFrame>
  ),
  // 7 — Single star, diffraction spike (minimal modernist)
  () => (
    <LogoFrame num={7} name="Vega · Minimal">
      <MarkStar size={58} color="var(--accent)"/>
      <Wordmark size={56} weight={400} tracking={-0.005}>Astrophoto</Wordmark>
    </LogoFrame>
  ),
  // 8 — Wordmark only, all-caps tracked, with hairline rules (the "no mark" option)
  () => (
    <LogoFrame num={8} name="Wordmark only" mono>
      <div style={{display:"flex", alignItems:"center", gap:"14px", width:"100%"}}>
        <div style={{flex:1, height:"1px", background:"var(--border-strong)"}}/>
        <div style={{
          fontFamily:"var(--font-display)",
          fontSize:"32px",
          letterSpacing:"0.32em",
          textTransform:"uppercase",
          fontWeight: 400,
          color:"var(--fg-primary)",
        }}>Astrophoto</div>
        <div style={{flex:1, height:"1px", background:"var(--border-strong)"}}/>
      </div>
      <div style={{
        fontFamily:"var(--font-mono)",
        fontSize:"10px",
        letterSpacing:"0.4em",
        textTransform:"uppercase",
        color:"var(--fg-muted)",
        marginTop:"-4px"
      }}>RA · DEC · INTEGRATION · LIGHT</div>
    </LogoFrame>
  ),
];

// Export the primary mark for reuse in nav etc.
window.AstroMarks = { MarkUrsa, MarkOrion, MarkReticle, MarkCassiopeia, MarkLyra, MarkAtlas, MarkStar, MarkMonogram, Wordmark };
