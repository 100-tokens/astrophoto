import { describe, it, expect, vi } from 'vitest';
import { Pump, makeUploadRunner } from './pump';

describe('Pump', () => {
  it('respects concurrency limit', async () => {
    let active = 0;
    let peak = 0;
    const calls: number[] = [];

    const pump = new Pump({
      concurrency: 2,
      runSlot: async (id: string) => {
        active++;
        peak = Math.max(peak, active);
        await new Promise((r) => setTimeout(r, 10));
        active--;
        calls.push(parseInt(id, 10));
      }
    });

    pump.add('1');
    pump.add('2');
    pump.add('3');
    pump.add('4');
    await pump.drain();
    expect(peak).toBeLessThanOrEqual(2);
    expect(calls.sort()).toEqual([1, 2, 3, 4]);
  });

  it('cancel() calls onCancel and skips runSlot', async () => {
    const onCancel = vi.fn();
    const runSlot = vi.fn();
    const pump = new Pump({ concurrency: 2, runSlot, onCancel });
    pump.add('1');
    pump.cancel('1');
    await pump.drain();
    expect(runSlot).not.toHaveBeenCalled();
    expect(onCancel).toHaveBeenCalledWith('1');
  });

  it('add() during drain picks up new work', async () => {
    const calls: string[] = [];
    const pump = new Pump({
      concurrency: 1,
      runSlot: async (id) => {
        calls.push(id);
        if (id === '1') pump.add('2');
        await Promise.resolve();
      }
    });
    pump.add('1');
    await pump.drain();
    expect(calls).toEqual(['1', '2']);
  });

  it('runner short-circuits on already-aborted signal', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');
    const handle = {
      slot: { name: 'a.jpg', size: 1, mime: 'image/jpeg', hash: 'h', file: new File([], 'a.jpg') },
      abort: new AbortController(),
      setProgress: vi.fn()
    };
    handle.abort.abort();
    const runner = makeUploadRunner(() => handle);
    await runner('1');
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(handle.setProgress).toHaveBeenCalledWith({ state: 'cancelled', pct: 0 });
    fetchSpy.mockRestore();
  });
});
