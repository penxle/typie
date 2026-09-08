/* eslint-disable @typescript-eslint/consistent-type-definitions */

import 'unplugin-icons/types/svelte';

declare global {
  namespace App {
    interface Platform {
      env?: {
        API_ORIGIN?: string;
      };
    }
  }
}
