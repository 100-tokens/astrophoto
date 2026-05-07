// PATCHes /api/photos/<id>/metadata with the latest snapshot, debounced.
// Exposes reactive state for the "● Saved 2s ago" indicator.

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

export type SaveState = 'idle' | 'saving' | 'error';

type Snapshot = Record<string, unknown>;

export class Saver {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private latest: Snapshot | null = null;
  private inFlight = false;
  state: SaveState = 'idle';
  lastSavedAt: number | null = null;
  lastError: string | null = null;

  constructor(
    private photoId: string,
    private fetchImpl: typeof fetch = fetch,
    private debounceMs: number = 800
  ) {}

  get label(): string {
    if (this.state === 'saving') return '● Saving…';
    if (this.state === 'error')
      return `● Save failed${this.lastError ? `: ${this.lastError}` : ''}`;
    if (this.lastSavedAt == null) return '';
    const ago = Math.max(1, Math.round((Date.now() - this.lastSavedAt) / 1000));
    return `● Saved ${ago}s ago`;
  }

  queue(snapshot: Snapshot): void {
    this.latest = snapshot;
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.flush(), this.debounceMs);
  }

  private async flush(): Promise<void> {
    if (this.inFlight) {
      this.timer = setTimeout(() => this.flush(), 50);
      return;
    }
    if (!this.latest) return;
    const body = this.latest;
    this.latest = null;
    this.inFlight = true;
    this.state = 'saving';
    try {
      const r = await this.fetchImpl(`${API}/api/photos/${this.photoId}/metadata`, {
        method: 'PATCH',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body)
      });
      if (!r.ok) {
        this.state = 'error';
        this.lastError = `${r.status}`;
        return;
      }
      this.state = 'idle';
      this.lastSavedAt = Date.now();
      this.lastError = null;
    } catch (e) {
      this.state = 'error';
      this.lastError = e instanceof Error ? e.message : String(e);
    } finally {
      this.inFlight = false;
      if (this.latest) this.flush();
    }
  }
}

/** Returns a Saver wrapped in a runes-friendly state proxy. */
export function useAutosave(photoId: string) {
  const inner = new Saver(photoId);
  let state = $state<SaveState>('idle');
  let label = $state<string>('');

  // Republish on a 250ms tick — cheap, avoids deep reactivity wiring.
  const interval = setInterval(() => {
    state = inner.state;
    label = inner.label;
  }, 250);

  return {
    queue: (snap: Snapshot) => inner.queue(snap),
    get state() {
      return state;
    },
    get label() {
      return label;
    },
    dispose: () => clearInterval(interval)
  };
}
