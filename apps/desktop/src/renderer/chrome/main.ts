import '../app.css';

import { init } from '@sentry/electron/renderer';
import { mount } from 'svelte';
import App from './App.svelte';

init();

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
mount(App, { target: document.querySelector('#app')! });
