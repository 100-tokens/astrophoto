// Small helpers used across the catalog screens. We use the existing
// AppHeader from shared.jsx; this file only adds primitives that aren't
// already in the system.

// Field — exact same pattern as screens-2.jsx ScreenUpload.
function Field({ label, value, mono = true, detected, full, children, hint, defaultValue }) {
  return (
    <div style={{ gridColumn: full ? '1 / -1' : 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginBottom: 6 }}>
        <span className="t-label">{label}</span>
        {detected && <span className="t-meta" style={{ color: detected === 'auto' ? 'var(--fg-muted)' : 'var(--accent)' }}>{detected === 'auto' ? 'YOU FILL' : 'FROM EXIF'}</span>}
      </div>
      {children ?? <input className={'input ' + (mono ? 'input-mono' : '')} defaultValue={value ?? defaultValue} />}
      {hint && <div className="t-meta" style={{ marginTop: 6 }}>{hint}</div>}
    </div>
  );
}

// Crumbs — uppercase mono breadcrumbs in the system's vocabulary.
function Crumbs({ trail }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 8,
      fontFamily: 'var(--font-mono)', fontSize: 11,
      letterSpacing: '0.10em', textTransform: 'uppercase',
      color: 'var(--fg-muted)',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
    }}>
      {trail.map((c, i) => (
        <React.Fragment key={i}>
          {i > 0 && <span style={{ color: 'var(--fg-faint)' }}>/</span>}
          {i === trail.length - 1
            ? <span style={{ color: 'var(--fg-primary)', whiteSpace: 'nowrap' }}>{c.label}</span>
            : <a href="#" style={{ color: 'var(--fg-muted)', whiteSpace: 'nowrap' }}>{c.label}</a>}
        </React.Fragment>
      ))}
    </div>
  );
}

// Subnav — an in-page secondary nav row using nav-link styling. Used on
// the equipment browse / settings surfaces. Active item is amber-underlined.
function SubNav({ items, active }) {
  return (
    <nav style={{ display: 'flex', gap: 32, height: 44, alignItems: 'center' }}>
      {items.map(it => (
        <a key={it.key} className={'nav-link' + (it.key === active ? ' active' : '')} href="#">
          {it.label}
          {it.count != null && (
            <span style={{ marginLeft: 8, fontSize: 10, color: 'var(--fg-faint)', letterSpacing: '0.06em' }}>
              {it.count.toLocaleString()}
            </span>
          )}
        </a>
      ))}
    </nav>
  );
}

// Inline callout — info/warning/accent flavored. Matches phase8 "tint
// section" pattern.
function Callout({ tone = 'accent', label, children }) {
  const bgVar = { accent: 'bg-accent-tint', warning: 'bg-warning-tint', info: 'bg-info-tint', success: 'bg-success-tint' }[tone];
  const fgVar = { accent: 'accent', warning: 'warning', info: 'info', success: 'success' }[tone];
  return (
    <div style={{
      padding: 16,
      background: `var(--${bgVar})`,
      border: `1px solid var(--border-${fgVar})`,
      fontSize: 12,
    }}>
      {label && (
        <div className="t-label" style={{ color: `var(--${fgVar})`, marginBottom: 8 }}>
          ● {label}
        </div>
      )}
      <div style={{ color: 'var(--fg-secondary)', fontSize: 13 }}>{children}</div>
    </div>
  );
}

// Synthesized deep-sky thumbnail — used for grids where window.PHOTOS doesn't
// have enough variety. Same recipe as design-system/components.html .ph.
function PlaceholderPhoto({ seed = 1, ratio = '4/3', style = {} }) {
  // deterministic hash from seed → pick one of 4 palettes
  let s = ((seed * 2654435761) >>> 0);
  const palettes = [
    ['rgba(220,140,60,.55)', 'rgba(60,120,160,.45)', 'rgba(180,80,80,.4)'],   // SHO warm
    ['rgba(170,100,180,.45)', 'rgba(60,160,180,.40)', 'rgba(240,180,80,.35)'], // narrowband cool
    ['rgba(232,164,58,.55)',  'rgba(120,80,200,.30)', 'rgba(180,80,60,.45)'],  // amber+violet
    ['rgba(80,160,120,.45)',  'rgba(232,164,58,.40)', 'rgba(200,120,80,.4)'],  // bicolor
  ];
  const p = palettes[s % palettes.length];
  return (
    <div style={{
      position: 'relative',
      aspectRatio: ratio,
      overflow: 'hidden',
      background: '#050507',
      ...style,
    }}>
      <div style={{
        position: 'absolute', inset: 0,
        background:
          `radial-gradient(ellipse 60% 45% at 35% 45%, ${p[0]}, transparent 65%),` +
          `radial-gradient(ellipse 45% 35% at 65% 60%, ${p[1]}, transparent 65%),` +
          `radial-gradient(ellipse 90% 60% at 50% 70%, ${p[2]}, transparent 75%)`,
      }} />
      <div style={{
        position: 'absolute', inset: 0,
        backgroundImage: 'radial-gradient(white 0.5px, transparent 0.6px)',
        backgroundSize: '18px 18px',
        opacity: 0.35,
      }} />
    </div>
  );
}

Object.assign(window, { Field, Crumbs, SubNav, Callout, PlaceholderPhoto });
