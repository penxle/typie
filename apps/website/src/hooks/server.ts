import './common';

import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import { GlobalWindow } from 'happy-dom';
import type { HandleServerError } from '@sveltejs/kit';

globalThis.__happydom__ = { window: new GlobalWindow() };

const log = logger.getChild('http');

export const handle = sequence(logging);

export const handleError: HandleServerError = ({ error, status, message }) => {
  log.error('Server error {*}', { status, message, error });
};
