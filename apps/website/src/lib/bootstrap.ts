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
  const resp = await fetch('/api/bootstrap').catch((err: unknown) => {
    if (err instanceof TypeError) {
      error(503, { code: 'network_error', message: '서버에 연결할 수 없어요.' });
    }
    throw err;
  });
  const body = await resp.json().catch(() => null);
  if (body?.code === 'maintenance') {
    error(resp.status, body);
  }
  if (resp.status === 502 || resp.status === 503 || resp.status === 504) {
    error(resp.status, { code: 'service_unavailable', message: '서버를 일시적으로 사용할 수 없어요.' });
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
