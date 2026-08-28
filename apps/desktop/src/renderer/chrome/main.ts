import '../app.css';

import { init } from '@sentry/electron/renderer';
import { mount } from 'svelte';
import App from './App.svelte';

init();

const params = new URLSearchParams(location.search);
for (const [key, name] of [
  ['theme', 'theme'],
  ['variantLight', 'variantLight'],
  ['variantDark', 'variantDark'],
] as const) {
  const value = params.get(key);
  if (value) document.documentElement.dataset[name] = value;
}

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
mount(App, { target: document.querySelector('#app')! });
