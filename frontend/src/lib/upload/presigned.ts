// API base follows the same convention as HandlePicker.svelte and cdn.ts.
const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

export type FileSlot = {
	name: string;
	size: number;
	mime: string;
	hash: string;
	file: File;
};

export type SlotProgress = {
	state: 'queued' | 'hashing' | 'uploading' | 'finalizing' | 'ready' | 'failed';
	pct: number;
	photoId?: string;
	shortId?: string;
	reason?: string;
};

export type Listener = (idx: number, p: SlotProgress) => void;

/**
 * Upload all slots to S3 via presigned PUT URLs, then finalize each slot.
 *
 * Flow:
 *   1. POST /api/uploads/init  (credentials: 'include' — auth cookie)
 *   2. Per slot: XHR PUT to presigned S3 URL (no credentials — the URL is
 *      the credential; cookies on a cross-origin S3 PUT are rejected by S3).
 *   3. Per slot: POST /api/uploads/<id>/finalize  (credentials: 'include')
 *
 * Concurrency is limited to 3 simultaneous slots.
 */
export async function uploadAll(slots: FileSlot[], listener: Listener): Promise<void> {
	const init = await fetch(`${API}/api/uploads/init`, {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			files: slots.map((s) => ({ name: s.name, size: s.size, mime: s.mime, hash: s.hash }))
		})
	});

	if (!init.ok) {
		const reason = await init.text();
		slots.forEach((_, i) => listener(i, { state: 'failed', pct: 0, reason }));
		return;
	}

	const body = (await init.json()) as {
		files: { photo_id: string; short_id: string; presigned_put_url: string }[];
	};

	// Build a shared queue of jobs then run 3 concurrent workers that each
	// drain from it. This gives us a concurrency limit of 3 without a semaphore.
	//
	// noUncheckedIndexedAccess makes body.files[idx] typed as T | undefined.
	// A mismatch between init response length and slot count is a server bug;
	// treat it the same as a failed init.
	const queue = slots.flatMap((slot, idx) => {
		const signed = body.files[idx];
		if (!signed) {
			listener(idx, { state: 'failed', pct: 0, reason: 'Server returned fewer presigned URLs than files' });
			return [];
		}
		return [{ slot, idx, signed }];
	});

	async function worker(): Promise<void> {
		while (queue.length > 0) {
			const job = queue.shift();
			if (!job) return;
			await uploadOne(job.idx, job.slot, job.signed, listener);
		}
	}

	await Promise.all(Array.from({ length: 3 }, () => worker()));
}

async function uploadOne(
	idx: number,
	slot: FileSlot,
	signed: { photo_id: string; short_id: string; presigned_put_url: string },
	listener: Listener
): Promise<void> {
	listener(idx, { state: 'uploading', pct: 0, photoId: signed.photo_id, shortId: signed.short_id });

	// XHR is used instead of fetch because fetch does not expose upload progress
	// in any browser. xhr.upload.onprogress gives byte-level feedback.
	//
	// XHR's withCredentials defaults to false, which is exactly what we want:
	// the cross-origin PUT to S3 must NOT send cookies — the presigned URL
	// itself is the credential. (Confirmed via Chrome DevTools test against real
	// S3: adding cookies to the PUT causes AWS to reject the request.)
	await new Promise<void>((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		xhr.open('PUT', signed.presigned_put_url);
		xhr.setRequestHeader('content-type', slot.mime);

		// wire handlers before send()
		xhr.upload.onprogress = (e) => {
			// Guard: some servers omit Content-Length so lengthComputable is false.
			if (e.lengthComputable) {
				listener(idx, {
					state: 'uploading',
					pct: (e.loaded / e.total) * 100,
					photoId: signed.photo_id,
					shortId: signed.short_id
				});
			}
		};

		xhr.onerror = () => reject(new Error('PUT failed'));
		xhr.onload = () => {
			if (xhr.status >= 200 && xhr.status < 300) {
				resolve();
			} else {
				reject(new Error(`PUT ${xhr.status}`));
			}
		};

		xhr.send(slot.file);
	}).catch((err: unknown) => {
		const reason = err instanceof Error ? err.message : String(err);
		listener(idx, { state: 'failed', pct: 0, reason, photoId: signed.photo_id, shortId: signed.short_id });
		throw err;
	});

	listener(idx, {
		state: 'finalizing',
		pct: 100,
		photoId: signed.photo_id,
		shortId: signed.short_id
	});

	const fin = await fetch(`${API}/api/uploads/${signed.photo_id}/finalize`, {
		method: 'POST',
		credentials: 'include'
	});

	if (!fin.ok) {
		listener(idx, {
			state: 'failed',
			pct: 100,
			reason: await fin.text(),
			photoId: signed.photo_id,
			shortId: signed.short_id
		});
		return;
	}

	listener(idx, { state: 'ready', pct: 100, photoId: signed.photo_id, shortId: signed.short_id });
}
