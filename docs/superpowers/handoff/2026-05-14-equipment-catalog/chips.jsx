// FilterChip + FilterChipInput — semantic heart of the catalog feature.
// Rebuilt on top of the Astrophoto design system (styles.css).

const { useState, useRef, useEffect, useMemo } = React;

function bandwidthLabel(f) {
  if (f.bandwidth_nm == null) return null;
  const t = f.filter_type;
  if (t === 'luminance' || t === 'uv_ir_cut' || t === 'broadband_color' || t === 'light_pollution') return null;
  return `${f.bandwidth_nm} nm`;
}

function FilterChip({ filter, draggable = false, removable = false, onRemove, compact = false, dragging = false }) {
  const types = window.CATALOG.filterTypes;
  const t = filter.filter_type ? types[filter.filter_type] : null;
  const isUntyped = !t;
  const bw = bandwidthLabel(filter);
  const cls =
    'fchip ' +
    (isUntyped ? 'is-untyped' : 'is-' + filter.filter_type) +
    (dragging ? ' is-dragging' : '');

  return (
    <span className={cls} title={t ? `${t.label}${bw ? ' · ' + bw : ''}` : 'Untyped filter — add a type'}>
      <span className="fchip-badge">{t ? t.code : '?'}</span>
      <span className="fchip-name">{filter.display_name}</span>
      {bw && !compact && <span className="fchip-bw">{bw}</span>}
      {isUntyped && !compact && (
        <a className="fchip-addtype" href="#" onClick={(e) => e.preventDefault()}>+ type</a>
      )}
      {draggable && (
        <span className="fchip-grip" title="Drag to reorder">
          <svg width="8" height="12" viewBox="0 0 8 12" fill="currentColor">
            <circle cx="1.5" cy="2"  r="0.9"/><circle cx="6.5" cy="2"  r="0.9"/>
            <circle cx="1.5" cy="6"  r="0.9"/><circle cx="6.5" cy="6"  r="0.9"/>
            <circle cx="1.5" cy="10" r="0.9"/><circle cx="6.5" cy="10" r="0.9"/>
          </svg>
        </span>
      )}
      {removable && (
        <button className="fchip-x" onClick={onRemove} title="Remove">
          <svg width="9" height="9" viewBox="0 0 9 9" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round">
            <path d="M2 2 L7 7 M7 2 L2 7"/>
          </svg>
        </button>
      )}
    </span>
  );
}

function FilterChipInput({ value, onChange, orphans = [], placeholder = 'Search filters…', startOpen = false }) {
  const [items, setItems] = useState(value || []);
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(startOpen);
  const [focusIdx, setFocusIdx] = useState(0);
  const [dragId, setDragId] = useState(null);
  const inputRef = useRef(null);

  useEffect(() => { setItems(value || []); }, [value]);

  const selectedIds = useMemo(() => new Set(items.map(i => i.id)), [items]);
  const all = window.CATALOG.filters;
  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    return all
      .filter(f => !selectedIds.has(f.id))
      .filter(f => !q || f.display_name.toLowerCase().includes(q) || (f.filter_type && window.CATALOG.filterTypes[f.filter_type].label.toLowerCase().includes(q)))
      .slice(0, 8);
  }, [query, selectedIds, all]);

  function add(f) {
    const next = [...items, f];
    setItems(next);
    onChange && onChange(next);
    setQuery('');
    setFocusIdx(0);
  }
  function remove(id) {
    const next = items.filter(f => f.id !== id);
    setItems(next);
    onChange && onChange(next);
  }
  function reorder(srcId, beforeId) {
    if (srcId === beforeId) return;
    const src = items.find(f => f.id === srcId);
    const rest = items.filter(f => f.id !== srcId);
    const idx = beforeId == null ? rest.length : rest.findIndex(f => f.id === beforeId);
    const next = [...rest.slice(0, idx), src, ...rest.slice(idx)];
    setItems(next);
    onChange && onChange(next);
  }

  function onKey(e) {
    if (e.key === 'ArrowDown') { e.preventDefault(); setFocusIdx(i => Math.min(matches.length - 1, i + 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setFocusIdx(i => Math.max(0, i - 1)); }
    else if (e.key === 'Enter' && matches[focusIdx]) { e.preventDefault(); add(matches[focusIdx]); }
    else if (e.key === 'Backspace' && !query && items.length) {
      remove(items[items.length - 1].id);
    } else if (e.key === 'Escape') { setOpen(false); }
  }

  return (
    <div className="fchip-input" onClick={() => { inputRef.current && inputRef.current.focus(); setOpen(true); }}>
      {items.map((f) => (
        <span
          key={f.id}
          draggable
          onDragStart={(e) => { setDragId(f.id); e.dataTransfer.effectAllowed = 'move'; }}
          onDragOver={(e) => { e.preventDefault(); }}
          onDrop={(e) => { e.preventDefault(); if (dragId) { reorder(dragId, f.id); setDragId(null); } }}
          onDragEnd={() => setDragId(null)}
          style={{ display: 'inline-flex' }}
        >
          <FilterChip filter={f} draggable removable dragging={dragId === f.id} onRemove={() => remove(f.id)} />
        </span>
      ))}
      {orphans.map((tok, i) => (
        <span key={'o' + i} className="fchip-orphan" title="Legacy text filter — no catalog match yet">
          <span className="lbl">legacy</span>{tok}
        </span>
      ))}
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => { setQuery(e.target.value); setOpen(true); setFocusIdx(0); }}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        onKeyDown={onKey}
        placeholder={items.length ? '' : placeholder}
      />
      {open && (
        <div className="fchip-pop" onMouseDown={(e) => e.preventDefault()}>
          <div className="fchip-pop-head">
            <span>{query ? `MATCHES "${query}"` : 'POPULAR FILTERS'}</span>
            <span style={{ color: 'var(--fg-faint)' }}>{matches.length} OF {all.length - selectedIds.size}</span>
          </div>
          <div className="fchip-pop-list">
            {matches.length === 0 && (
              <div style={{ padding: '14px', color: 'var(--fg-muted)', fontSize: '12px', fontFamily: 'var(--font-mono)' }}>
                No matches. Press <span style={{ color: 'var(--accent)' }}>↵ Enter</span> to create a new filter item.
              </div>
            )}
            {matches.map((f, i) => {
              const t = f.filter_type ? window.CATALOG.filterTypes[f.filter_type] : null;
              const bw = bandwidthLabel(f);
              return (
                <div
                  key={f.id}
                  className={'fchip-pop-item' + (i === focusIdx ? ' is-focus' : '')}
                  onClick={() => add(f)}
                  onMouseEnter={() => setFocusIdx(i)}
                >
                  <FilterChip filter={f} compact />
                  <span className="meta">{t ? `${t.label.toUpperCase()}${bw ? ' · ' + bw : ''}` : 'UNTYPED'}</span>
                  <span className="usage">{f.usage_count.toLocaleString()} PHOTOS</span>
                </div>
              );
            })}
          </div>
          {query && (
            <div className="fchip-pop-create">
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.4"><path d="M6 2v8M2 6h8" strokeLinecap="round"/></svg>
              <span>Create new · "<strong style={{ color: 'var(--fg-primary)' }}>{query}</strong>"</span>
              <span className="kbd">↵ ENTER</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

Object.assign(window, { FilterChip, FilterChipInput, bandwidthLabel });
