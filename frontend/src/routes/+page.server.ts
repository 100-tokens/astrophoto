import { PHOTOS, NGC7000 } from '$lib/data/photos';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
	return {
		photos: PHOTOS.slice(0, 12),
		heroTarget: NGC7000.target,
		heroTargetSubtitle: NGC7000.targetSubtitle,
		heroIntegration: NGC7000.integration,
		heroPhotographer: NGC7000.photographer.name,
		heroBortle: NGC7000.photographer.bortle
	};
};
