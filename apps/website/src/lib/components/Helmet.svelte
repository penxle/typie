<script lang="ts">
  import { page } from '$app/state';

  type Props = {
    type?: string;
    title: string;
    trailing?: string | null;
    description?: string;
    image?: string | { src: string; size: 'small' | 'large' };
    struct?: Record<string, unknown>;
  };

  let { type = 'website', title, trailing = '타이피', description, image, struct }: Props = $props();

  const href = $derived(`https://${page.url.host}${page.url.pathname}`);
  const effectiveTitle = $derived(trailing ? `${title}${trailing ? ` · ${trailing}` : ''}` : title);
</script>

<svelte:head>
  <title>{effectiveTitle}</title>
  <meta content={effectiveTitle} property="og:title" />
  {#if description}
    <meta name="description" content={description} />
    <meta content={description} property="og:description" />
  {/if}
  {#if typeof image === 'string'}
    <meta content={image} property="og:image" />
    <meta content="summary" property="twitter:card" />
  {:else if typeof image === 'object'}
    <meta content={image.src} property="og:image" />
    {#if image.size === 'large'}
      <meta content="summary_large_image" property="twitter:card" />
    {:else}
      <meta content="summary" property="twitter:card" />
    {/if}
  {/if}
  <meta content={href} property="og:url" />
  <meta content="타이피" property="og:site_name" />
  <meta content={type} property="og:type" />
  <meta content="ko_KR" property="og:locale" />
  <link {href} rel="canonical" />
  {#if struct}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html '<' + `script type="application/ld+json">${JSON.stringify(struct)}</script>`}
  {/if}
</svelte:head>
