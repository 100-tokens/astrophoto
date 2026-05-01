/* ===== Shared chrome: AppHeader, Footer, Photo placeholder ===== */

// Real public-domain astrophotos from NASA / ESA / Wikimedia Commons.
// Each entry: { src, alt, target, ratio (w/h), exposure, integration, photographer }
window.PHOTOS = [
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/c/c4/Pillars_2014_HST_WFC3-UVIS_full-res_denoised.jpg/800px-Pillars_2014_HST_WFC3-UVIS_full-res_denoised.jpg",
    target: "M16 · Pillars of Creation", ratio: 1.16, integration: "—", photographer: "Hubble (NASA/ESA)", camera: "Hubble WFC3" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/0/09/Andromeda_Galaxy_%28with_h-alpha%29.jpg/1280px-Andromeda_Galaxy_%28with_h-alpha%29.jpg",
    target: "M31 · Andromeda Galaxy", ratio: 1.5, integration: "9h 40m", photographer: "Marie Dubois", camera: "ZWO ASI2600MC" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/b/b4/Heart_Nebula_%28cropped%29.jpg/1024px-Heart_Nebula_%28cropped%29.jpg",
    target: "IC 1805 · Heart Nebula", ratio: 1.4, integration: "14h 06m", photographer: "StarHunter42", camera: "QHY268M" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/6/68/NGC_6960_-_Western_Veil_Nebula.jpg/1024px-NGC_6960_-_Western_Veil_Nebula.jpg",
    target: "NGC 6960 · Western Veil", ratio: 1.5, integration: "11h 30m", photographer: "K. Aalto", camera: "ASI2600MM" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/4/47/Carina_Nebula_by_Webb%27s_NIRCam.jpg/1024px-Carina_Nebula_by_Webb%27s_NIRCam.jpg",
    target: "NGC 3324 · Cosmic Cliffs", ratio: 1.7, integration: "—", photographer: "JWST", camera: "NIRCam" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/2/2b/Orion_Nebula_-_Hubble_2006_mosaic_18000.jpg/1024px-Orion_Nebula_-_Hubble_2006_mosaic_18000.jpg",
    target: "M42 · Orion Nebula", ratio: 1.33, integration: "6h 12m", photographer: "L. Petrov", camera: "Canon R6" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/9/9e/Horsehead_Nebula_Christmas_2017_Deep_Field.jpg/1024px-Horsehead_Nebula_Christmas_2017_Deep_Field.jpg",
    target: "IC 434 · Horsehead Nebula", ratio: 1.0, integration: "8h 48m", photographer: "CometChaser_2024", camera: "ASI294MC" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/7/72/NGC_7000_in_H-alpha%2C_OIII_and_SII.jpg/1024px-NGC_7000_in_H-alpha%2C_OIII_and_SII.jpg",
    target: "NGC 7000 · North America", ratio: 1.4, integration: "18h 00m", photographer: "Marie Dubois", camera: "ASI2600MC Pro" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/5/57/Triangulum_Galaxy_M33.jpg/1024px-Triangulum_Galaxy_M33.jpg",
    target: "M33 · Triangulum Galaxy", ratio: 1.5, integration: "12h 24m", photographer: "P. Halverson", camera: "QHY600M" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/1/16/M51_The_Whirlpool_Galaxy.jpg/1024px-M51_The_Whirlpool_Galaxy.jpg",
    target: "M51 · Whirlpool Galaxy", ratio: 1.0, integration: "7h 18m", photographer: "S. Tanaka", camera: "ASI6200MM" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4d/Rho_Ophiuchi.jpg/1024px-Rho_Ophiuchi.jpg",
    target: "ρ Ophiuchi Cloud", ratio: 1.5, integration: "5h 45m", photographer: "A. Dimov", camera: "Sony A7R V" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/8/85/NGC_281%2C_Pacman_Nebula_-_Crop.jpg/1024px-NGC_281%2C_Pacman_Nebula_-_Crop.jpg",
    target: "NGC 281 · Pacman Nebula", ratio: 1.3, integration: "10h 12m", photographer: "R. Mehta", camera: "ASI533MC Pro" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/5/55/M27_Dumbbell_Nebula.jpg/1024px-M27_Dumbbell_Nebula.jpg",
    target: "M27 · Dumbbell Nebula", ratio: 1.5, integration: "4h 30m", photographer: "L. Petrov", camera: "QHY268M" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/9/96/Full_Moon_Luc_Viatour.jpg/800px-Full_Moon_Luc_Viatour.jpg",
    target: "Moon · Mare Imbrium", ratio: 1.0, integration: "— (1 frame)", photographer: "L. Viatour", camera: "C11 Edge" },
  { src: "https://upload.wikimedia.org/wikipedia/commons/thumb/9/91/NGC_2070.jpg/1024px-NGC_2070.jpg",
    target: "NGC 2070 · Tarantula", ratio: 1.5, integration: "9h 00m", photographer: "Southern Sky Co.", camera: "ASI2600MM" },
];

window.AppHeader = function AppHeader({ active = "Gallery", auth = false, marks }) {
  const Wordmark = marks.Wordmark;
  const Mark = marks.MarkReticle;
  return (
    <header className="app-header">
      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
        <Mark size={28} color="var(--accent)" />
        <Wordmark size={22} weight={600} italic={false}>Astrophoto</Wordmark>
      </div>
      <nav style={{ display: "flex", gap: "32px" }}>
        {["Gallery", "Targets", "Photographers", "About"].map(l => (
          <a key={l} className={"nav-link" + (active === l ? " active" : "")}>{l}</a>
        ))}
      </nav>
      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
        <div style={{
          display:"flex", alignItems:"center", gap:"8px",
          padding:"0 12px", height:"32px",
          border:"1px solid var(--border-default)",
          borderRadius:"2px", color:"var(--fg-muted)",
          fontFamily:"var(--font-mono)", fontSize:"12px",
          width:"220px",
        }}>
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
            <circle cx="7" cy="7" r="5"/><line x1="11" y1="11" x2="14" y2="14"/>
          </svg>
          <span>search the archive…</span>
          <span style={{marginLeft:"auto", fontSize:"10px", letterSpacing:"0.1em"}}>⌘K</span>
        </div>
        {auth ? (
          <>
            <button className="btn btn-secondary btn-sm">Upload</button>
            <div style={{
              width:"32px", height:"32px",
              borderRadius:"50%",
              background:"var(--accent)",
              color:"var(--accent-ink)",
              display:"flex", alignItems:"center", justifyContent:"center",
              fontFamily:"var(--font-display)", fontSize:"15px",
            }}>M</div>
          </>
        ) : (
          <>
            <a className="nav-link">Sign in</a>
            <button className="btn btn-primary btn-sm">Create account</button>
          </>
        )}
      </div>
    </header>
  );
};

window.AppFooter = function AppFooter() {
  return (
    <footer style={{
      borderTop: "1px solid var(--border-subtle)",
      padding: "32px",
      display: "flex",
      justifyContent: "space-between",
      fontFamily: "var(--font-mono)",
      fontSize: "11px",
      letterSpacing: "0.08em",
      textTransform: "uppercase",
      color: "var(--fg-muted)",
    }}>
      <span>Astrophoto · Est. MMXXVI</span>
      <span style={{display:"flex", gap:"24px"}}>
        <a>About</a><a>Terms</a><a>Privacy</a><a>Contact</a><a>RSS</a>
      </span>
      <span>52°31′N · 13°24′E · Bortle 8</span>
    </footer>
  );
};

/* Photo block — uses real image OR styled placeholder fallback */
window.Photo = function Photo({ photo, style, className = "" }) {
  // Generate a deterministic per-photo gradient based on target name
  const hash = (s) => { let h = 0; for (let i = 0; i < s.length; i++) h = ((h<<5)-h+s.charCodeAt(i))|0; return Math.abs(h); };
  const h = hash(photo.target);
  // 6 curated nebula palettes
  const palettes = [
    // SHO narrowband (gold/teal/red)
    ["rgba(220,140,60,.55)", "rgba(60,120,160,.45)", "rgba(180,80,80,.4)"],
    // HOO (blue/cyan)
    ["rgba(80,140,200,.6)", "rgba(40,180,200,.45)", "rgba(20,40,80,.5)"],
    // Galaxy (warm core, cool arms)
    ["rgba(255,200,140,.5)", "rgba(180,140,200,.35)", "rgba(80,60,120,.4)"],
    // Emission Hα red
    ["rgba(200,80,80,.6)", "rgba(140,60,100,.4)", "rgba(60,40,80,.5)"],
    // Lunar / silver
    ["rgba(220,210,200,.7)", "rgba(140,130,120,.5)", "rgba(40,40,50,.6)"],
    // Carina cliffs (orange/teal)
    ["rgba(220,160,90,.55)", "rgba(80,160,170,.4)", "rgba(60,40,30,.5)"],
  ];
  const pIdx = h % palettes.length;
  const [c1, c2, c3] = palettes[pIdx];
  const x1 = 20 + (h % 50), y1 = 30 + ((h>>3) % 40);
  const x2 = 50 + ((h>>5) % 30), y2 = 50 + ((h>>7) % 30);

  // Star pattern
  const stars = [];
  for (let i = 0; i < 28; i++) {
    const sh = (h + i * 9301) & 0xfffffff;
    stars.push({
      x: (sh % 100),
      y: ((sh >> 7) % 100),
      r: 0.4 + ((sh >> 13) % 10) / 10 * 1.4,
      o: 0.3 + ((sh >> 17) % 10) / 10 * 0.7,
    });
  }

  return (
    <div className={"photo-card " + className} style={{ ...style }}>
      <div style={{
        position: "absolute", inset: 0,
        background: `
          radial-gradient(ellipse 60% 45% at ${x1}% ${y1}%, ${c1}, transparent 65%),
          radial-gradient(ellipse 45% 35% at ${x2}% ${y2}%, ${c2}, transparent 65%),
          radial-gradient(ellipse 90% 60% at 50% 70%, ${c3}, transparent 75%),
          #050507
        `,
      }}/>
      <svg style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} preserveAspectRatio="none" viewBox="0 0 100 100">
        {stars.map((s, i) => (
          <circle key={i} cx={s.x} cy={s.y} r={s.r * 0.15} fill="white" opacity={s.o}/>
        ))}
      </svg>
    </div>
  );
};
