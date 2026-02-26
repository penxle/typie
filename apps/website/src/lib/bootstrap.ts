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

export async function checkBootstrapAssertion(fetch: typeof globalThis.fetch): Promise<void> {
  const resp = await fetch('/api/bootstrap');
  const body = await resp.json().catch(() => null);
  if (body?.code === 'maintenance') {
    error(resp.status, body);
  }
}

export function pollBootstrapAssertion(): () => void {
  const interval = setInterval(async () => {
    try {
      const resp = await fetch('/api/bootstrap');
      const body = await resp.json().catch(() => null);
      if (body?.code === 'maintenance') {
        location.reload();
      }
    } catch {
      // 네트워크 오류 무시
    }
  }, 60_000);
  return () => clearInterval(interval);
}
