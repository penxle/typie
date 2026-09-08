import { fetchPage } from './changelog';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => ({ initial: await fetchPage(1, fetch) });
