import 'unplugin-icons/types/svelte';

/* eslint-disable @typescript-eslint/consistent-type-definitions */
import type { D1Database, Workflow } from '@cloudflare/workers-types';

declare global {
  namespace App {
    interface Locals {
      email: string;
    }
    interface Platform {
      env: {
        DB: D1Database;
        INGEST_TOKEN: string;
        DEV_EMAIL?: string;
        ADMIN_EMAILS?: string;
        INTERNAL_API_KEY: string;
        INTERNAL_API_BASE: string;
        SAMPLING: Workflow<{ runId: string; corpusVersion: string; size: number }>;
        PIPELINE: Workflow<{ runId: string; promptVariantId: string; variantLabel: string; corpusVersion: string; documentId: string }>;
      };
    }
  }
}
