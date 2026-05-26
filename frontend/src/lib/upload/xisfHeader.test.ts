import { describe, it, expect } from 'vitest';
import { parseXisfHeader } from './xisfHeader';

// base64 of little-endian f64 values (PCL:TotalExposureTime shape).
function f64leB64(values: number[]): string {
  const dv = new DataView(new ArrayBuffer(values.length * 8));
  values.forEach((v, i) => dv.setFloat64(i * 8, v, true));
  let bin = '';
  for (const b of new Uint8Array(dv.buffer)) bin += String.fromCharCode(b);
  return btoa(bin);
}

// Wrap XML in a minimal XISF monolithic header: "XISF0100" + u32 LE header
// length + 4 reserved bytes + the XML.
function makeXisf(xml: string): File {
  const xmlBytes = new TextEncoder().encode(xml);
  const buf = new Uint8Array(16 + xmlBytes.length);
  buf.set(new TextEncoder().encode('XISF0100'), 0);
  new DataView(buf.buffer).setUint32(8, xmlBytes.length, true);
  buf.set(xmlBytes, 16);
  return new File([buf], 'master.xisf', { type: 'application/x-xisf' });
}

describe('parseXisfHeader', () => {
  it('extracts filter, frames, and total exposure', async () => {
    const xml = `<xisf><Image geometry="100:100:1">
      <FITSKeyword name="FILTER" value="'L'" comment="filter"/>
      <FITSKeyword name="NCOMBINE" value="120"/>
      <Property id="PCL:TotalExposureTime" format="f64" value="${f64leB64([14400])}"/>
    </Image></xisf>`;
    expect(await parseXisfHeader(makeXisf(xml))).toEqual({
      filter: 'L',
      frames: 120,
      totalExposureS: 14400,
      subExposureS: null
    });
  });

  it('reads per-sub exposure from EXPTIME (real WBPP masterLight shape)', async () => {
    const xml = `<xisf><Image geometry="3008:3008:1">
      <FITSKeyword name="FILTER" value="'L'" comment="Filter used when taking image"/>
      <FITSKeyword name="EXPTIME" value="15.00" comment="Exposure time in seconds"/>
    </Image></xisf>`;
    const out = await parseXisfHeader(makeXisf(xml));
    expect(out).toEqual({ filter: 'L', frames: null, totalExposureS: null, subExposureS: 15 });
  });

  it('sums a multi-channel TotalExposureTime vector', async () => {
    const xml = `<xisf><Property id="PCL:TotalExposureTime" value="${f64leB64([4800, 4800, 4800])}"/></xisf>`;
    const out = await parseXisfHeader(makeXisf(xml));
    expect(out?.totalExposureS).toBe(14400);
  });

  it('falls back to Process:Integration:ImageCount for frames', async () => {
    const xml = `<xisf><Property id="Process:Integration:ImageCount" value="40"/></xisf>`;
    const out = await parseXisfHeader(makeXisf(xml));
    expect(out?.frames).toBe(40);
  });

  it('reads filter from PCL when no FITS FILTER keyword', async () => {
    const xml = `<xisf><Property id="Instrument:Filter:Name" value="Ha"/></xisf>`;
    const out = await parseXisfHeader(makeXisf(xml));
    expect(out?.filter).toBe('Ha');
  });

  it('returns nulls when fields are absent', async () => {
    const out = await parseXisfHeader(makeXisf(`<xisf><Image geometry="10:10:1"/></xisf>`));
    expect(out).toEqual({ filter: null, frames: null, totalExposureS: null, subExposureS: null });
  });

  it('returns null for a non-XISF file (bad signature)', async () => {
    const f = new File([new Uint8Array([0x42, 0x4d, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])], 'x.bmp');
    expect(await parseXisfHeader(f)).toBeNull();
  });

  it('returns null on malformed base64 total exposure (not 8-byte aligned)', async () => {
    const xml = `<xisf><Property id="PCL:TotalExposureTime" value="${btoa('xyz')}"/></xisf>`;
    const out = await parseXisfHeader(makeXisf(xml));
    expect(out?.totalExposureS).toBeNull();
  });
});
