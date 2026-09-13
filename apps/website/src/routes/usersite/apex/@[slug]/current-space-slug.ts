import { page } from '$app/state';

export const currentSpaceSlug = () => page.params.slug ?? '';
