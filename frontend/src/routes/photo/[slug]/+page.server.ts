import { error } from '@sveltejs/kit';
import { PHOTOS, NGC7000 } from '$lib/data/photos';
import type { PageServerLoad } from './$types';
import type { PhotoDetail } from '$lib/data/photos';

/** Minimal photo detail shape for gallery photos that lack rich EXIF data. */
function minimalDetail(
	target: string,
	integration: string,
	photographerName: string,
	slug: string,
	ratio: number
): PhotoDetail {
	return {
		slug,
		target,
		targetSubtitle: '',
		captured: '',
		camera: '',
		cameraSub: '',
		telescope: '',
		telescopeSub: '',
		mount: '',
		filters: '',
		exposure: '',
		exposureTotal: '',
		gain: '',
		ra: '',
		dec: '',
		field: '',
		pixelScale: '',
		publishedDate: '',
		photographer: {
			name: photographerName,
			initial: photographerName.charAt(0).toUpperCase(),
			frames: 0,
			bortle: 0,
			location: '',
			caption: ''
		},
		appreciations: 0,
		comments: 0,
		ratio,
		integration
	};
}

export const load: PageServerLoad = async ({ params }) => {
	const { slug } = params;

	if (slug === 'ngc-7000-north-america-nebula') {
		return { photo: NGC7000, isRich: true };
	}

	const match = PHOTOS.find((p) => p.slug === slug);
	if (!match) {
		throw error(404, 'Photo not found');
	}

	return {
		photo: minimalDetail(
			match.target,
			match.integration,
			match.photographer,
			match.slug,
			match.ratio
		),
		isRich: false
	};
};
