import { assertBootstrap } from '$lib/bootstrap';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ fetch, depends, getClientAddress }) => {
  depends('app:bootstrap');
  await assertBootstrap(fetch, getClientAddress());
  return {};
};
