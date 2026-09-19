import '../app.css';

import { mount } from 'svelte';
import App from './App.svelte';

const params = new URLSearchParams(location.search);
for (const key of ['theme', 'variantLight', 'variantDark']) {
  const value = params.get(key);
  if (value) document.documentElement.dataset[key] = value;
}

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
mount(App, { target: document.querySelector('#app')! });
