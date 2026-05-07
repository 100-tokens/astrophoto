/** Format right ascension (decimal degrees) as sexagesimal hours: "00ʰ42ᵐ44ˢ". */
export function formatRA(degrees: number): string {
  const norm = ((degrees % 360) + 360) % 360;
  const hoursTotal = norm / 15;
  const h = Math.floor(hoursTotal);
  const minutesTotal = (hoursTotal - h) * 60;
  const m = Math.floor(minutesTotal);
  const s = Math.round((minutesTotal - m) * 60);
  return `${pad(h)}ʰ${pad(m)}ᵐ${pad(s)}ˢ`;
}

/** Format declination (decimal degrees) as signed sexagesimal: "+41°16′09″". */
export function formatDec(degrees: number): string {
  const sign = degrees < 0 ? '-' : '+';
  const abs = Math.abs(degrees);
  const d = Math.floor(abs);
  const minutesTotal = (abs - d) * 60;
  const min = Math.floor(minutesTotal);
  const sec = Math.round((minutesTotal - min) * 60);
  return `${sign}${pad(d)}°${pad(min)}′${pad(sec)}″`;
}

function pad(n: number): string {
  return n.toString().padStart(2, '0');
}
