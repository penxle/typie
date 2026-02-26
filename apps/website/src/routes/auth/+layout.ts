import { checkBootstrapAssertion } from '$lib/bootstrap';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch }) => {
  await checkBootstrapAssertion(fetch);
  return {};
};
