<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import qs from 'query-string';
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { thumbHashToDataURL } from 'thumbhash';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { HTMLImgAttributes } from 'svelte/elements';

  type Size = 16 | 24 | 32 | 48 | 64 | 96 | 128 | 256 | 512 | 1024 | 'full';

  type Props = Omit<HTMLImgAttributes, 'style' | 'src' | 'srcset' | 'sizes' | 'alt' | 'placeholder'> & {
    url: string;
    alt: string;
    style?: SystemStyleObject;
    size: Size;
    ratio?: number;
    quality?: number;
    placeholder?: string;
    progressive?: boolean;
  };

  let { url, alt, style, size, ratio, quality, placeholder, progressive = false, ...rest }: Props = $props();

  let containerEl = $state<HTMLElement>();
  let targetEl = $state<HTMLElement>();

  let loaded = $state(false);

  const isVideo = $derived(url.endsWith('.mp4'));

  const src = $derived(qs.stringifyUrl({ url, query: { s: size === 'full' ? undefined : size, q: quality } }));
  const src2x = $derived(size !== 'full' && qs.stringifyUrl({ url, query: { s: size * 2, q: quality } }));
  const src3x = $derived(size !== 'full' && qs.stringifyUrl({ url, query: { s: size * 3, q: quality } }));

  const sizes = $derived(size === 'full' ? undefined : `${size}px`);
  const srcset = $derived(size === 'full' ? undefined : `${src} ${size}w, ${src2x} ${size * 2}w, ${src3x} ${size * 3}w`);

  const placeholderUrl = $derived(placeholder ? thumbHashToDataURL(Uint8Array.fromBase64(placeholder)) : undefined);

  const load = () => {
    if (loaded) {
      return;
    }

    if (isVideo) {
      const videoEl = document.createElement('video');

      videoEl.addEventListener('loadeddata', async () => {
        loaded = true;
        await tick();
        // eslint-disable-next-line svelte/no-dom-manipulating
        targetEl?.append(videoEl);
        videoEl.play();
      });

      videoEl.className = css({ size: 'full', objectFit: 'cover' }, style);
      videoEl.autoplay = true;
      videoEl.loop = true;
      videoEl.muted = true;
      videoEl.playsInline = true;
      videoEl.disablePictureInPicture = true;
      videoEl.src = url;
    } else {
      const imgEl = document.createElement('img');

      imgEl.addEventListener('load', async () => {
        loaded = true;
        await tick();
        // eslint-disable-next-line svelte/no-dom-manipulating
        targetEl?.append(imgEl);
      });

      if (srcset && sizes) {
        imgEl.sizes = sizes;
        imgEl.srcset = srcset;
      }

      imgEl.className = css({ size: 'full', objectFit: 'cover' }, style);
      imgEl.alt = alt;
      imgEl.src = src;

      Object.assign(imgEl, rest);
    }
  };

  $effect(() => {
    if (containerEl && progressive) {
      loaded = false;

      const observer = new IntersectionObserver((entries) => {
        if (entries.every((entry) => !entry.isIntersecting)) {
          return;
        }

        load();
        observer.disconnect();
      });

      observer.observe(containerEl);

      return () => {
        observer.disconnect();
      };
    }
  });
</script>

{#key url}
  {#if progressive && placeholderUrl}
    <div
      bind:this={containerEl}
      style:aspect-ratio={ratio}
      class={css({
        position: 'relative',
        width: 'full',
        overflow: 'hidden',
      })}
    >
      <img class={css(style, { size: 'full', objectFit: 'cover' })} {alt} loading="lazy" src={placeholderUrl} {...rest} />

      {#if loaded}
        <div
          bind:this={targetEl}
          class={css({ position: 'absolute', inset: '0', backgroundColor: 'surface.default' })}
          in:fade={{ duration: 200 }}
        ></div>
      {/if}
    </div>
  {:else}
    {#if isVideo}
      <video
        class={css(style, { size: 'full', objectFit: 'cover' })}
        autoplay
        disablepictureinpicture
        loop
        muted
        onloadeddata={(e) => {
          loaded = true;
          e.currentTarget.play();
        }}
        playsinline
        src={url}
      ></video>
    {:else}
      <img
        class={css(style)}
        {alt}
        {sizes}
        {src}
        {srcset}
        {...rest}
        onload={() => {
          loaded = true;
        }}
      />
    {/if}
    {#if !loaded && placeholderUrl}
      <img class={css(style, { size: 'full', objectFit: 'cover' })} {alt} src={placeholderUrl} {...rest} />
    {/if}
  {/if}
{/key}
