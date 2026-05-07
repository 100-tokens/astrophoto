import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Saver } from './useAutosave.svelte';

describe('Saver', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('debounces calls within the window', async () => {
    const fetchMock: typeof fetch = vi.fn(async () => new Response(null, { status: 200 }));
    const s = new Saver('photo-1', fetchMock, 800);

    s.queue({ caption: 'a' });
    vi.advanceTimersByTime(400);
    s.queue({ caption: 'b' });
    vi.advanceTimersByTime(400);
    s.queue({ caption: 'c' });
    vi.advanceTimersByTime(800);
    await Promise.resolve();
    await Promise.resolve();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [, init] = (fetchMock as ReturnType<typeof vi.fn>).mock.calls[0]!;
    const lastCallBody = JSON.parse((init as RequestInit).body as string);
    expect(lastCallBody.caption).toBe('c');
  });

  it('marks state=error on non-2xx response', async () => {
    const fetchMock = vi.fn(async () => new Response('boom', { status: 500 }));
    const s = new Saver('photo-1', fetchMock, 100);
    s.queue({ caption: 'x' });
    vi.advanceTimersByTime(150);
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
    expect(s.state).toBe('error');
  });
});
