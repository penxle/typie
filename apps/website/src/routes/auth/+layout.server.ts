import { assertBootstrap } from '$lib/bootstrap.server';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ fetch, depends, getClientAddress, cookies }) => {
  depends('app:bootstrap');
  await assertBootstrap(fetch, getClientAddress(), cookies.get('typie-bb'));
  return {};
};
