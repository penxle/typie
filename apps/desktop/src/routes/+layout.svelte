<script lang="ts">
  import '../app.css';

  import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
  import { css } from '@typie/styled-system/css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  let { children } = $props();

  onMount(() => {
    onOpenUrl((urls) => {
      const url = new URL(urls[0]);
      goto(`${url.pathname}${url.search}`);
    });
  });
</script>

<div class={css({ position: 'relative', width: '[100vw]', height: '[100vh]', paddingTop: '36px' })}>
  <div
    style:-webkit-app-region="drag"
    class={css({ position: 'fixed', top: '0', left: '0', right: '0', height: '36px' })}
    data-tauri-drag-region
  ></div>

  {@render children()}
</div>
