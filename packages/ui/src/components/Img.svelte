<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import qs from 'query-string';
  import { base64 } from 'rfc4648';
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { thumbHashToDataURL } from 'thumbhash';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { HTMLImgAttributes } from 'svelte/elements';

  type Size = 16 | 24 | 32 | 48 | 64 | 96 | 128 | 256 | 512 | 1024 | 'full';

  type Props = {
    url: string;
    alt: string;
    style?: SystemStyleObject;
    size: Size;
    ratio?: number;
    quality?: number;
    placeholder?: string;
  } & Omit<HTMLImgAttributes, 'style' | 'src' | 'srcset' | 'sizes' | 'alt' | 'placeholder'>;

  let { url, alt, style, size, ratio, quality, placeholder, ...rest }: Props = $props();

  let containerEl = $state<HTMLElement>();
  let targetEl = $state<HTMLElement>();

  let loaded = $state(false);

  const src = $derived(qs.stringifyUrl({ url, query: { s: size === 'full' ? undefined : size, q: quality } }));
  const src2x = $derived(size !== 'full' && qs.stringifyUrl({ url, query: { s: size * 2, q: quality } }));
  const src3x = $derived(size !== 'full' && qs.stringifyUrl({ url, query: { s: size * 3, q: quality } }));

  const sizes = $derived(size === 'full' ? undefined : `${size}px`);
  const srcset = $derived(size === 'full' ? undefined : `${src} ${size}w, ${src2x} ${size * 2}w, ${src3x} ${size * 3}w`);

  const placeholderUrl = $derived(placeholder ? thumbHashToDataURL(base64.parse(placeholder)) : undefined);

  const load = () => {
    if (loaded) {
      return;
    }

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
  };

  $effect(() => {
    if (containerEl) {
      loaded = false;

      const observer = new IntersectionObserver((entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          load();
          observer.disconnect();
        }
      });

      observer.observe(containerEl);

      return () => {
        observer.disconnect();
      };
    }
  });
</script>

{#key url}
  {#if placeholderUrl}
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
    <img class={css(style)} {alt} {sizes} {src} {srcset} {...rest} />
  {/if}
{/key}
