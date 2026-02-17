<script lang="ts" module>
  import { SvelteMap } from 'svelte/reactivity';

  const cache = new SvelteMap<string, string | null>();
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import ky, { HTTPError } from 'ky';
  import { untrack } from 'svelte';
  import { env } from '$env/dynamic/public';

  type Props = {
    fontId?: string | null;
    text: string;
    weight?: number;
  };

  let { fontId = null, text, weight }: Props = $props();

  const key = $derived(fontId ? `${fontId}:${text}` : null);
  const html = $derived(key ? cache.get(key) : null);

  $effect(() => {
    if (!key || cache.has(key)) return;

    const loadSpecimen = async () => {
      try {
        const svg = await ky(`${env.PUBLIC_API_URL}/font/${fontId}/specimen`, { searchParams: { text } }).text();
        cache.set(key, svg);
      } catch (err) {
        if (err instanceof HTTPError && err.response.status === 422) {
          cache.set(key, null);
        }
      }
    };

    untrack(() => loadSpecimen());
  });
</script>

{#if html}
  <span class={css({ display: 'inline-flex', alignItems: 'center', height: '[1lh]', '& > svg': { height: '[1em]', width: 'auto' } })}>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html html}
  </span>
{:else}
  <span style:font-weight={weight}>{text}</span>
{/if}
