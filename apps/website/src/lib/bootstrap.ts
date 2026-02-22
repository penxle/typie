import { error } from '@sveltejs/kit';

export type MaintenanceConfig = {
  enabled: boolean;
  title: string;
  message: string;
  until: string | null;
  platforms: string[];
  allowedIps: string[];
};

export type Bootstrap = {
  version: number;
  updatedAt: string;
  maintenance: MaintenanceConfig;
};

export async function assertBootstrap(fetch: typeof globalThis.fetch, clientIp: string) {
  const resp = await fetch('/api/bootstrap');
  if (!resp.ok) {
    return;
  }

  const bootstrap: Bootstrap | null = await resp.json();
  if (
    bootstrap?.maintenance.enabled &&
    bootstrap.maintenance.platforms.includes('web') &&
    !bootstrap.maintenance.allowedIps.includes(clientIp)
  ) {
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
}
