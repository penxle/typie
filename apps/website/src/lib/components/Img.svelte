<script lang="ts">
  import qs from 'query-string';
  import { base64 } from 'rfc4648';
  import { tick } from 'svelte';
  import { fade } from 'svelte/transition';
  import { thumbHashToDataURL } from 'thumbhash';
  import { fragment, graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import type { HTMLImgAttributes } from 'svelte/elements';
  import type { Img_image } from '$graphql';
  import type { SystemStyleObject } from '$styled-system/types';

  type Size = 16 | 24 | 32 | 48 | 64 | 96 | 128 | 256 | 512 | 1024 | 'full';

  type Props = {
    $image: Img_image;
    alt: string;
    style?: SystemStyleObject;
    size: Size;
    quality?: number;
    progressive?: boolean;
  } & Omit<HTMLImgAttributes, 'style' | 'src' | 'srcset' | 'sizes' | 'alt'>;

  let { $image: _image, alt, style, size, quality, progressive = false, ...rest }: Props = $props();

  const image = fragment(
    _image,
    graphql(`
      fragment Img_image on Image {
        id
        url
        ratio
        placeholder
      }
    `),
  );

  let containerEl = $state<HTMLElement>();
  let targetEl = $state<HTMLElement>();

  let loaded = $state(false);

  const src = $derived(qs.stringifyUrl({ url: $image.url, query: { s: size === 'full' ? undefined : size, q: quality } }));
  const src2x = $derived(size !== 'full' && qs.stringifyUrl({ url: $image.url, query: { s: size * 2, q: quality } }));
  const src3x = $derived(size !== 'full' && qs.stringifyUrl({ url: $image.url, query: { s: size * 3, q: quality } }));

  const sizes = $derived(size === 'full' ? undefined : `${size}px`);
  const srcset = $derived(size === 'full' ? undefined : `${src} ${size}w, ${src2x} ${size * 2}w, ${src3x} ${size * 3}w`);

  const placeholderUrl = $derived(progressive ? thumbHashToDataURL(base64.parse($image.placeholder)) : undefined);

  const load = () => {
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

{#if placeholderUrl}
  <div
    bind:this={containerEl}
    style:aspect-ratio={$image.ratio}
    class={css({
      position: 'relative',
      width: 'full',
      overflow: 'hidden',
    })}
  >
    <img class={css(style, { size: 'full', objectFit: 'cover' })} {alt} loading="lazy" src={placeholderUrl} {...rest} />

    {#if loaded}
      <div bind:this={targetEl} class={css({ position: 'absolute', inset: '0' })} in:fade={{ duration: 200 }}></div>
    {/if}
  </div>
{:else}
  <img class={css(style)} {alt} loading="lazy" {sizes} {src} {srcset} {...rest} />
{/if}
