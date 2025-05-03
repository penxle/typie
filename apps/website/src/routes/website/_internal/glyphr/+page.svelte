<script lang="ts">
  import * as Comlink from 'comlink';
  import { browser } from '$app/environment';
  import { css } from '$styled-system/css';
  import Worker from './worker?worker';
  import type { WorkerApi } from './worker';

  let canvasEl = $state<HTMLCanvasElement>();

  let previewText = $state('typie? 타이피!');

  const worker = browser ? Comlink.wrap<WorkerApi>(new Worker()) : null;

  const fonts = [
    { name: 'KoPubWorldBatang', url: 'https://cdn.typie.net/fonts/ttf/KoPubWorldBatang-Medium.ttf' },
    { name: 'KoPubWorldDotum', url: 'https://cdn.typie.net/fonts/ttf/KoPubWorldDotum-Medium.ttf' },
    { name: 'NanumBarunGothic', url: 'https://cdn.typie.net/fonts/ttf/NanumBarunGothic-Regular.ttf' },
    { name: 'NanumMyeongjo', url: 'https://cdn.typie.net/fonts/ttf/NanumMyeongjo-Regular.ttf' },
    { name: 'Pretendard', url: 'https://cdn.typie.net/fonts/ttf/Pretendard-Regular.ttf' },
    { name: 'RIDIBatang', url: 'https://cdn.typie.net/fonts/ttf/RIDIBatang-Regular.ttf' },
  ];

  const loadFont = async (url?: string) => {
    const response = await fetch(url ?? fonts[0].url);
    const arrayBuffer = await response.arrayBuffer();
    const data = new Uint8Array(arrayBuffer);

    await worker?.loadFont(data);
    await worker?.renderText(previewText);
  };

  const init = async () => {
    if (canvasEl) {
      const offscreen = canvasEl.transferControlToOffscreen();
      await worker?.init(Comlink.transfer(offscreen, [offscreen]));
      await loadFont(fonts[0].url);
    }
  };

  $effect(() => {
    if (canvasEl) {
      init();
    }

    return () => {
      worker?.free();
    };
  });

  $effect(() => {
    worker?.renderText(previewText);
  });
</script>

<div class={css({ maxWidth: '780px', marginX: 'auto', paddingX: '20px', paddingY: '40px' })}>
  <div
    class={css({
      display: 'flex',
      flexDirection: 'column',
      gap: '4px',
    })}
  >
    <select
      class={css({
        paddingX: '16px',
        paddingY: '10px',
        borderWidth: '1px',
        borderColor: 'gray.200',
        borderRadius: '6px',
        fontSize: '14px',
        backgroundColor: 'white',
        color: 'gray.700',
        width: 'full',
      })}
      onchange={(e) => loadFont(e.currentTarget.value)}
    >
      {#each fonts as font (font.name)}
        <option value={font.url}>{font.name}</option>
      {/each}
    </select>

    <input
      class={css({
        paddingX: '16px',
        paddingY: '10px',
        borderWidth: '1px',
        borderColor: 'gray.200',
        borderRadius: '6px',
        fontSize: '14px',
        backgroundColor: 'white',
        color: 'gray.800',
        width: 'full',
      })}
      placeholder="렌더링할 텍스트"
      type="text"
      bind:value={previewText}
    />

    <canvas
      bind:this={canvasEl}
      class={css({
        width: '740px',
        height: '260px',
        border: '1px solid',
        borderColor: 'gray.200',
        borderRadius: '6px',
        backgroundColor: 'white',
      })}
      height="260"
      width="740"
    ></canvas>
  </div>
</div>
