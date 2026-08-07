import 'unplugin-icons/types/svelte';

/* eslint-disable @typescript-eslint/consistent-type-definitions */
import type { D1Database } from '@cloudflare/workers-types';

declare global {
  namespace App {
    interface Locals {
      email: string;
    }
    interface Platform {
      env: {
        DB: D1Database;
        DEV_EMAIL?: string;
        ADMIN_EMAILS?: string;
        INTERNAL_API_KEY: string;
        INTERNAL_API_BASE: string;
        PRISM_API_ORIGIN: string;
        PRISM_API_TOKEN: string;
      };
      // 응답 반환 후에도 이어서 돌릴 백그라운드 작업. dev 프록시에는 없을 수 있어 선택형으로 둔다.
      context?: { waitUntil(promise: Promise<unknown>): void };
    }
  }
}
