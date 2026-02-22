import { createHash } from 'node:crypto';
import { error } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

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

export async function assertBootstrap(fetch: typeof globalThis.fetch, clientIp: string, bypassKeyHash?: string) {
  const resp = await fetch('/api/bootstrap');
  if (!resp.ok) {
    return;
  }

  const bootstrap: Bootstrap | null = await resp.json();
  if (
    bootstrap?.maintenance.enabled &&
    bootstrap.maintenance.platforms.includes('web') &&
    !['127.0.0.1', '::1', ...bootstrap.maintenance.allowedIps].includes(clientIp)
  ) {
    if (env.PRIVATE_BOOTSTRAP_BYPASS_KEY) {
      const expectedHash = createHash('sha256').update(env.PRIVATE_BOOTSTRAP_BYPASS_KEY).digest('hex');
      if (bypassKeyHash === expectedHash) {
        return;
      }
    }

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
