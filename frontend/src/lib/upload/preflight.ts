import { parse as parseExif } from 'exifr';

export type Preflight = {
	thumbDataUrl: string;
	exif: Record<string, unknown>;
	hash: string;
};

export async function preflight(file: File): Promise<Preflight> {
	const [thumbDataUrl, exif, hash] = await Promise.all([
		makeThumb(file),
		parseExif(file).catch(() => ({})),
		sha256(file)
	]);
	return { thumbDataUrl, exif: exif ?? {}, hash };
}

async function makeThumb(file: File): Promise<string> {
	const bmp = await createImageBitmap(file, { resizeWidth: 256, resizeQuality: 'medium' });
	const canvas = document.createElement('canvas');
	canvas.width = bmp.width;
	canvas.height = bmp.height;
	canvas.getContext('2d')!.drawImage(bmp, 0, 0);
	return canvas.toDataURL('image/jpeg', 0.8);
}

async function sha256(file: File): Promise<string> {
	if (typeof crypto !== 'undefined' && crypto.subtle?.digest) {
		const buf = await file.arrayBuffer();
		const digest = await crypto.subtle.digest('SHA-256', buf);
		return [...new Uint8Array(digest)]
			.map((b) => b.toString(16).padStart(2, '0'))
			.join('');
	}
	// Insecure-context fallback: deterministic but weak. Sufficient for dedup
	// in dev when accessing via raw IP. Production runs on HTTPS so this is dead path.
	const stub = `${file.size}-${file.lastModified}-${file.name}`;
	return stub
		.split('')
		.map((c) => c.charCodeAt(0).toString(16).padStart(2, '0'))
		.join('')
		.slice(0, 64)
		.padEnd(64, '0');
}
