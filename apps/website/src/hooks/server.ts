import './common';

import { sequence } from '@sveltejs/kit/hooks';
import { logger, logging } from '@typie/lib/svelte';
import { GlobalWindow } from 'happy-dom';
import type { Handle, HandleServerError } from '@sveltejs/kit';

globalThis.__happydom__ = { window: new GlobalWindow() };

const log = logger.getChild('http');

const theme: Handle = async ({ event, resolve }) => {
  const theme = event.cookies.get('typie-th');

  return resolve(event, {
    transformPageChunk: ({ html }) => {
      if (event.url.pathname.includes('landing')) {
        return html.replace('%app.theme%', 'light');
      }

      const defaultTheme = event.url.pathname.includes('_webview') ? 'light' : 'auto';
      const themeValue = theme && ['auto', 'light', 'dark'].includes(theme) ? theme : defaultTheme;
      return html.replace('%app.theme%', themeValue);
    },
  });
};

const header: Handle = async ({ event, resolve }) => {
  return resolve(event, {
    filterSerializedResponseHeaders: (name) => {
      const n = name.toLowerCase();

      if (n === 'content-type') {
        return true;
      }

      return false;
    },
  });
};

export const handle = sequence(logging, theme, header);

export const handleError: HandleServerError = ({ error, status, message }) => {
  log.error('Server error {*}', { status, message, error });
};
