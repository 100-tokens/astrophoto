export type PumpOptions = {
  concurrency: number;
  runSlot: (id: string) => Promise<void>;
  onCancel?: (id: string) => void;
};

export class Pump {
  private queue: string[] = [];
  private cancelled = new Set<string>();
  private active = 0;
  private waiters: Array<() => void> = [];

  constructor(private opts: PumpOptions) {}

  add(id: string): void {
    this.queue.push(id);
    this.tryDispatch();
  }

  cancel(id: string): void {
    this.cancelled.add(id);
    this.opts.onCancel?.(id);
  }

  /** Resolves once the queue is empty AND no slots are running. */
  async drain(): Promise<void> {
    while (this.queue.length > 0 || this.active > 0) {
      await new Promise<void>((r) => this.waiters.push(r));
    }
  }

  private notifyDone(): void {
    const w = this.waiters.shift();
    w?.();
    this.waiters.forEach((fn) => fn());
    this.waiters = [];
  }

  private tryDispatch(): void {
    while (this.active < this.opts.concurrency && this.queue.length > 0) {
      const id = this.queue.shift()!;
      if (this.cancelled.has(id)) {
        this.notifyDone();
        continue;
      }
      this.active++;
      // Defer to a microtask so cancel() called synchronously after add()
      // can mark the id before runSlot fires.
      queueMicrotask(() => {
        if (this.cancelled.has(id)) {
          this.active--;
          this.notifyDone();
          this.tryDispatch();
          return;
        }
        Promise.resolve(this.opts.runSlot(id))
          .catch(() => {})
          .finally(() => {
            this.active--;
            this.notifyDone();
            this.tryDispatch();
          });
      });
    }
    if (this.active === 0 && this.queue.length === 0) {
      this.notifyDone();
    }
  }
}

export type FileSlot = {
  name: string;
  size: number;
  mime: string;
  hash: string;
  file: File;
};

export type SlotProgress = {
  state: 'queued' | 'hashing' | 'uploading' | 'finalizing' | 'ready' | 'failed' | 'cancelled';
  pct: number;
  photoId?: string;
  shortId?: string;
  reason?: string;
};

export type SlotHandle = {
  slot: FileSlot;
  abort: AbortController;
  setProgress: (p: SlotProgress) => void;
  signed?: { photo_id: string; short_id: string; presigned_put_url: string };
};

// API base follows the same convention as HandlePicker.svelte and cdn.ts.
const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

export function makeUploadRunner(getHandle: (id: string) => SlotHandle | undefined) {
  return async (id: string): Promise<void> => {
    const h = getHandle(id);
    if (!h) return;
    const { slot, abort, setProgress } = h;

    if (abort.signal.aborted) {
      setProgress({ state: 'cancelled', pct: 0 });
      return;
    }

    setProgress({ state: 'uploading', pct: 0 });
    let signed: SlotHandle['signed'];
    try {
      const init = await fetch(`${API}/api/uploads/init`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ files: [{ name: slot.name, size: slot.size, mime: slot.mime, hash: slot.hash }] }),
        signal: abort.signal,
      });
      if (!init.ok) throw new Error(await init.text());
      const json = (await init.json()) as { files: NonNullable<SlotHandle['signed']>[] };
      const first = json.files[0];
      if (!first) throw new Error('init: server returned no files');
      signed = first;
      h.signed = signed;
    } catch (e) {
      if (abort.signal.aborted) {
        setProgress({ state: 'cancelled', pct: 0 });
      } else {
        const reason = e instanceof Error ? e.message : String(e);
        setProgress({ state: 'failed', pct: 0, reason });
      }
      return;
    }

    setProgress({ state: 'uploading', pct: 0, photoId: signed.photo_id, shortId: signed.short_id });

    try {
      await xhrPut(slot, signed.presigned_put_url, abort.signal, (pct) =>
        setProgress({ state: 'uploading', pct, photoId: signed!.photo_id, shortId: signed!.short_id })
      );
    } catch (e) {
      if (abort.signal.aborted) {
        setProgress({ state: 'cancelled', pct: 0, photoId: signed.photo_id, shortId: signed.short_id });
      } else {
        const reason = e instanceof Error ? e.message : 'PUT failed';
        setProgress({ state: 'failed', pct: 0, reason, photoId: signed.photo_id, shortId: signed.short_id });
      }
      return;
    }

    setProgress({ state: 'finalizing', pct: 100, photoId: signed.photo_id, shortId: signed.short_id });

    try {
      const fin = await fetch(`${API}/api/uploads/${signed.photo_id}/finalize`, {
        method: 'POST',
        credentials: 'include',
        signal: abort.signal,
      });
      if (!fin.ok) throw new Error(await fin.text());
    } catch (e) {
      if (abort.signal.aborted) {
        setProgress({ state: 'cancelled', pct: 100, photoId: signed.photo_id, shortId: signed.short_id });
      } else {
        const reason = e instanceof Error ? e.message : 'Finalize failed';
        setProgress({ state: 'failed', pct: 100, reason, photoId: signed.photo_id, shortId: signed.short_id });
      }
      return;
    }

    setProgress({ state: 'ready', pct: 100, photoId: signed.photo_id, shortId: signed.short_id });
  };
}

function xhrPut(
  slot: FileSlot,
  url: string,
  signal: AbortSignal,
  onProgress: (pct: number) => void
): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    if (signal.aborted) {
      reject(new DOMException('Aborted', 'AbortError'));
      return;
    }
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', url);
    xhr.setRequestHeader('content-type', slot.mime);
    const onAbort = () => xhr.abort();
    signal.addEventListener('abort', onAbort);
    xhr.upload.onprogress = (e) => {
      // Guard: some servers omit Content-Length so lengthComputable is false.
      if (e.lengthComputable) onProgress((e.loaded / e.total) * 100);
    };
    xhr.onerror = () => {
      signal.removeEventListener('abort', onAbort);
      reject(new Error('PUT failed'));
    };
    xhr.onabort = () => {
      signal.removeEventListener('abort', onAbort);
      reject(new DOMException('Aborted', 'AbortError'));
    };
    xhr.onload = () => {
      signal.removeEventListener('abort', onAbort);
      if (xhr.status >= 200 && xhr.status < 300) resolve();
      else reject(new Error(`PUT ${xhr.status}`));
    };
    xhr.send(slot.file);
  });
}
