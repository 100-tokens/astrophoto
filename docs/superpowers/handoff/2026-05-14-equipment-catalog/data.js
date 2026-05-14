// Real-world astrophotography gear catalog samples.
// Used across the four mocked surfaces.

window.CATALOG = {
  // Filter palette / typing — semantic colors per filter_type enum.
  // Order matches docs/superpowers/specs/2026-05-14-equipment-catalog-enriched.md
  filterTypes: {
    luminance:         { code: 'L',     label: 'Luminance',       color: '#f8f1e6' },
    red:               { code: 'R',     label: 'Red',             color: '#c25048' },
    green:             { code: 'G',     label: 'Green',           color: '#7da64a' },
    blue:              { code: 'B',     label: 'Blue',            color: '#6b8db8' },
    h_alpha:           { code: 'Hα',    label: 'Hydrogen α',      color: '#b04634' },
    oiii:              { code: 'OIII',  label: 'Oxygen III',      color: '#4ea0a8' },
    sii:               { code: 'SII',   label: 'Sulfur II',       color: '#e8a43a' },
    uv_ir_cut:         { code: 'UV/IR', label: 'UV / IR cut',     color: '#8a6a9c' },
    dual_band:         { code: 'D',     label: 'Dual-band',       color: '#7a8fa8' },
    tri_band:          { code: 'T',     label: 'Tri-band',        color: '#7a9588' },
    quad_band:         { code: 'Q',     label: 'Quad-band',       color: '#8a8a6a' },
    light_pollution:   { code: 'LP',    label: 'Light pollution', color: '#c98920' },
    broadband_color:   { code: 'BB',    label: 'Broadband color', color: '#d6cdba' },
    other:             { code: '?',     label: 'Other / untyped', color: '#6a6358' },
  },

  // Filter items (mix of typed and untyped, narrowband and broadband)
  filters: [
    { id: 'f1',  display_name: 'Antlia 3nm Hα Pro',         brand: 'Antlia',     filter_type: 'h_alpha', bandwidth_nm: 3,    size: '2in',    mounted: true,  usage_count: 1284 },
    { id: 'f2',  display_name: 'Antlia 3nm OIII Pro',       brand: 'Antlia',     filter_type: 'oiii',    bandwidth_nm: 3,    size: '2in',    mounted: true,  usage_count: 982 },
    { id: 'f3',  display_name: 'Antlia 3nm SII Pro',        brand: 'Antlia',     filter_type: 'sii',     bandwidth_nm: 3,    size: '2in',    mounted: true,  usage_count: 871 },
    { id: 'f4',  display_name: 'Chroma 3nm Hα',             brand: 'Chroma',     filter_type: 'h_alpha', bandwidth_nm: 3,    size: '36mm',   mounted: true,  usage_count: 612 },
    { id: 'f5',  display_name: 'Chroma 5nm OIII',           brand: 'Chroma',     filter_type: 'oiii',    bandwidth_nm: 5,    size: '36mm',   mounted: true,  usage_count: 548 },
    { id: 'f6',  display_name: 'Astronomik L-2 UV-IR Block',brand: 'Astronomik', filter_type: 'uv_ir_cut', size: '1_25in', mounted: false, usage_count: 2103 },
    { id: 'f7',  display_name: 'Astronomik Deep-Sky R',     brand: 'Astronomik', filter_type: 'red',     size: '1_25in', mounted: false, usage_count: 422 },
    { id: 'f8',  display_name: 'Astronomik Deep-Sky G',     brand: 'Astronomik', filter_type: 'green',   size: '1_25in', mounted: false, usage_count: 419 },
    { id: 'f9',  display_name: 'Astronomik Deep-Sky B',     brand: 'Astronomik', filter_type: 'blue',    size: '1_25in', mounted: false, usage_count: 421 },
    { id: 'f10', display_name: 'Baader L-Pro',              brand: 'Baader',     filter_type: 'luminance', size: '2in',  mounted: false, usage_count: 1671 },
    { id: 'f11', display_name: 'Optolong L-eXtreme',        brand: 'Optolong',   filter_type: 'dual_band', bandwidth_nm: 7, size: '2in',  mounted: true,  usage_count: 3940 },
    { id: 'f12', display_name: 'Optolong L-Ultimate',       brand: 'Optolong',   filter_type: 'dual_band', bandwidth_nm: 3, size: '2in',  mounted: true,  usage_count: 2117 },
    { id: 'f13', display_name: 'IDAS NBZ II',               brand: 'IDAS',       filter_type: 'dual_band', bandwidth_nm: 12, size: '2in', mounted: true,  usage_count: 1180 },
    { id: 'f14', display_name: 'Radian Triad Ultra',        brand: 'Radian',     filter_type: 'quad_band', bandwidth_nm: 5, size: '2in',  mounted: true,  usage_count: 760 },
    { id: 'f15', display_name: 'IDAS LPS-D3',               brand: 'IDAS',       filter_type: 'light_pollution', size: '2in', mounted: false, usage_count: 605 },
    { id: 'f16', display_name: 'Astronomik CLS',            brand: 'Astronomik', filter_type: null,      bandwidth_nm: null, size: null, mounted: null,  usage_count: 88 }, // intentionally untyped
  ],

  // Telescopes (some typed, one untyped)
  telescopes: [
    { id: 't1', display_name: 'Askar FRA400 75mm Quintuplet',  design: 'refractor_apo', aperture_mm: 75,  focal_length_mm: 400,  usage_count: 1820 },
    { id: 't2', display_name: 'William Optics RedCat 51 II',   design: 'refractor_apo', aperture_mm: 51,  focal_length_mm: 250,  usage_count: 2950 },
    { id: 't3', display_name: 'Sky-Watcher Esprit 100 ED',     design: 'refractor_apo', aperture_mm: 100, focal_length_mm: 550,  usage_count: 1560 },
    { id: 't4', display_name: 'Celestron EdgeHD 8"',           design: 'sct',           aperture_mm: 203, focal_length_mm: 2032, usage_count: 1140 },
    { id: 't5', display_name: 'Takahashi FSQ-106EDX4',         design: 'refractor_apo', aperture_mm: 106, focal_length_mm: 530,  usage_count: 870 },
  ],

  // Cameras
  cameras: [
    { id: 'c1', display_name: 'ZWO ASI2600MM Pro',  sensor_type: 'cmos', color_type: 'mono', cooled: true, sensor_model: 'Sony IMX571', pixel_size_um: 3.76, sensor_width_px: 6248, sensor_height_px: 4176, usage_count: 4250 },
    { id: 'c2', display_name: 'ZWO ASI2600MC Pro',  sensor_type: 'cmos', color_type: 'osc',  cooled: true, sensor_model: 'Sony IMX571', pixel_size_um: 3.76, sensor_width_px: 6248, sensor_height_px: 4176, usage_count: 5910 },
    { id: 'c3', display_name: 'QHY268M',            sensor_type: 'cmos', color_type: 'mono', cooled: true, sensor_model: 'Sony IMX571', pixel_size_um: 3.76, sensor_width_px: 6280, sensor_height_px: 4210, usage_count: 1130 },
    { id: 'c4', display_name: 'Player One Poseidon-M Pro', sensor_type: 'cmos', color_type: 'mono', cooled: true, sensor_model: 'Sony IMX571', pixel_size_um: 3.76, sensor_width_px: 6248, sensor_height_px: 4176, usage_count: 612 },
  ],

  mounts: [
    { id: 'm1', display_name: 'Sky-Watcher EQ6-R Pro', mount_type: 'equatorial_german', payload_kg: 20,  goto: true, usage_count: 3120 },
    { id: 'm2', display_name: 'ZWO AM5',               mount_type: 'harmonic_drive',    payload_kg: 13,  goto: true, usage_count: 2480 },
    { id: 'm3', display_name: 'iOptron CEM70',         mount_type: 'equatorial_german', payload_kg: 31,  goto: true, usage_count: 940 },
  ],

  modifiers: [
    { id: 'r1', display_name: 'Askar 0.7× Reducer for FRA400', modifier_type: 'reducer_flattener', factor: 0.70, usage_count: 740 },
    { id: 'r2', display_name: 'Celestron 0.7× Reducer EdgeHD 800', modifier_type: 'reducer_flattener', factor: 0.70, usage_count: 410 },
  ],

  // Sample photo set for the /equip/filter/antlia-ha-3nm browse page
  photosForAntliaHa: [
    { id: 'p1',  title: 'NGC 7000 — North America Nebula',     handle: 'pascal',   short: 'na7000',  integ_h: 14.5, bortle: 4 },
    { id: 'p2',  title: 'IC 1805 — Heart Nebula (SHO)',         handle: 'kestrel',  short: 'heart',   integ_h: 22.0, bortle: 5 },
    { id: 'p3',  title: 'M42 — Orion Nebula core',              handle: 'mlysander', short: 'm42c',  integ_h: 6.2,  bortle: 6 },
    { id: 'p4',  title: 'Sh2-155 — Cave Nebula',                handle: 'pascal',   short: 'cave',    integ_h: 9.8,  bortle: 4 },
    { id: 'p5',  title: 'IC 410 — Tadpoles in Hα',              handle: 'astro_naima', short: 'tads', integ_h: 11.3, bortle: 5 },
    { id: 'p6',  title: 'NGC 281 — Pacman Nebula',              handle: 'theo',     short: 'pac',     integ_h: 7.5,  bortle: 5 },
    { id: 'p7',  title: 'Sh2-132 — Lion Nebula bicolor',        handle: 'kestrel',  short: 'lion',    integ_h: 18.7, bortle: 4 },
    { id: 'p8',  title: 'M16 — Pillars of Creation',            handle: 'pascal',   short: 'm16',     integ_h: 10.0, bortle: 4 },
    { id: 'p9',  title: 'IC 1396 — Elephant\u2019s Trunk',      handle: 'mlysander', short: 'iet',   integ_h: 13.8, bortle: 6 },
    { id: 'p10', title: 'NGC 6888 — Crescent in narrowband',    handle: 'astro_naima', short: 'cre', integ_h: 16.0, bortle: 5 },
    { id: 'p11', title: 'Sh2-101 — Tulip Nebula',               handle: 'theo',     short: 'tul',     integ_h: 8.5,  bortle: 6 },
    { id: 'p12', title: 'NGC 1499 — California Nebula',         handle: 'pascal',   short: 'cal',     integ_h: 12.6, bortle: 4 },
  ],
};

// Pre-build a lookup for chip rendering
window.CATALOG.filterById = Object.fromEntries(window.CATALOG.filters.map(f => [f.id, f]));
