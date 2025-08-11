<script lang="ts">
  import '../app.css';

  import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check } from '@tauri-apps/plugin-updater';
  import { css } from '@typie/styled-system/css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  let { children } = $props();

  const checkForUpdates = async () => {
    const update = await check();
    if (!update) {
      return;
    }

    const result = await confirm('새로운 버전이 있어요.\n업데이트하시겠어요?', {
      kind: 'info',
      title: '업데이트 확인',
      okLabel: '지금 업데이트',
      cancelLabel: '나중에 하기',
    });

    if (!result) {
      return;
    }

    await update.downloadAndInstall();
    await relaunch();
  };

  onMount(() => {
    onOpenUrl((urls) => {
      const url = new URL(urls[0]);
      goto(`${url.pathname}${url.search}`);
    });

    checkForUpdates();
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
