import './common';

import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import type { HandleServerError } from '@sveltejs/kit';

const log = logger.getChild('http');

export const handle = sequence(logging);

export const handleError: HandleServerError = ({ error, status, message }) => {
  log.error`Server error (status: ${status}, message: ${message}): ${error}`;
};
