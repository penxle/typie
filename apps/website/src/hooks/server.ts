import './common';

import { sequence } from '@sveltejs/kit/hooks';
import { logging } from '@typie/lib/svelte';

export const handle = sequence(logging);
