import '../app.css';

import { mount } from 'svelte';
import App from './App.svelte';

const theme = new URLSearchParams(location.search).get('theme');
if (theme) document.documentElement.dataset.theme = theme;

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
mount(App, { target: document.querySelector('#app')! });
