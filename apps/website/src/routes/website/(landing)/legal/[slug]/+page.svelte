<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import Shell from '$lib/landing/components/Shell.svelte';
  import Markdown from '$lib/markdown/Markdown.svelte';
  import { parseMarkdown } from '$lib/markdown/parse';
  import type { PageData } from './$types';

  type Props = { data: PageData };

  let { data }: Props = $props();

  const blocks = $derived(parseMarkdown(data.document.body));

  const mainClass = css({
    width: 'full',
    maxWidth: { base: '[760px]', lg: '[800px]' },
    marginX: 'auto',
    paddingX: { base: '20px', lg: '40px' },
    paddingTop: { base: '112px', lg: '144px' },
    paddingBottom: { base: '96px', lg: '128px' },
  });
  const titleClass = css({
    fontSize: { base: '[26px]', md: '[32px]' },
    fontWeight: 'bold',
    lineHeight: '[1.3]',
    letterSpacing: '[-0.02em]',
  });
  const bodyClass = css({
    marginTop: { base: '32px', lg: '40px' },
    fontSize: { base: '15px', md: '16px' },
    color: 'text.default',
  });
</script>

<Helmet description={data.description} title={data.document.title} />

<Shell>
  <main class={mainClass}>
    <h1 class={titleClass}>{data.document.title}</h1>
    <article class={bodyClass}><Markdown {blocks} /></article>
  </main>
</Shell>
