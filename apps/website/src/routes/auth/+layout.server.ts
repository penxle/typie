import { error } from '@sveltejs/kit';
import type { Bootstrap } from '$lib/bootstrap';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ fetch, depends }) => {
  depends('app:bootstrap');

  const resp = await fetch('/api/bootstrap');
  if (!resp.ok) {
    return {};
  }

  const bootstrap: Bootstrap | null = await resp.json();
  if (bootstrap?.maintenance.enabled && bootstrap.maintenance.platforms.includes('web')) {
    error(503, {
      message: bootstrap.maintenance.message,
      code: 'maintenance',
      maintenance: {
        title: bootstrap.maintenance.title,
        message: bootstrap.maintenance.message,
        until: bootstrap.maintenance.until,
      },
    });
  }

  return {};
};
