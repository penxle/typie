import './common';

import { logging } from '@glitter/lib/svelte';
import { sequence } from '@sveltejs/kit/hooks';

export const handle = sequence(logging);
