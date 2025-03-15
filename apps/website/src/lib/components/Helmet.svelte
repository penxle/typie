<script lang="ts">
  import { page } from '$app/state';

  type Props = {
    type?: string;
    title: string;
    trailing?: string | null;
    description?: string;
    image?: string | { src: string; size: 'small' | 'large' };
  };

  let { type = 'website', title, trailing = '글리터', description, image }: Props = $props();

  const href = $derived(page.url.href);
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
  <link {href} rel="canonical" />
  <meta content={href} property="og:url" />
  <meta content="글리터" property="og:site_name" />
  <meta content={type} property="og:type" />
  <meta content="ko_KR" property="og:locale" />
</svelte:head>
