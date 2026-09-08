<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import Markdown from '$lib/markdown/Markdown.svelte';
  import { parseMarkdown } from '$lib/markdown/parse';
  import { formatDate } from './changelog';
  import Sentinel from './Sentinel.svelte';
  import type { Feed } from './feed.svelte';

  type Props = { feed: Feed };

  let { feed }: Props = $props();

  const listClass = css({ position: 'relative', maxWidth: '[960px]', marginX: 'auto', marginTop: { base: '56px', lg: '80px' } });
  const lineClass = css({
    display: { base: 'none', lg: 'block' },
    position: 'absolute',
    top: '[14px]',
    bottom: '[14px]',
    left: '[3px]',
    width: '1px',
    backgroundColor: 'border.hairline',
    pointerEvents: 'none',
  });
  const entryClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', lg: '[160px minmax(0, 1fr)]' },
    gap: { base: '12px', lg: '48px' },
    paddingY: { base: '40px', lg: '0' },
    paddingBottom: { lg: '72px' },
    _lastOfType: { paddingBottom: { lg: '0' } },
  });
  const dateColClass = css({
    position: { lg: 'sticky' },
    top: { lg: '96px' },
    alignSelf: 'start',
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
    height: { lg: '[31px]' },
    paddingTop: { lg: '5px' },
    paddingBottom: { lg: '8px' },
  });
  const dotClass = css({
    display: { base: 'none', lg: 'block' },
    flexShrink: '0',
    width: '7px',
    height: '7px',
    borderRadius: 'full',
    backgroundColor: 'border.emphasis',
  });
  const dateClass = css({
    fontFamily: 'mono',
    fontSize: '13px',
    letterSpacing: '[0.02em]',
    color: 'text.hint',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  });
  const titleClass = css({
    fontSize: { base: '[24px]', md: '[30px]' },
    fontWeight: 'bold',
    lineHeight: '[1.3]',
    letterSpacing: '[-0.02em]',
  });
  const imageClass = css({
    display: 'block',
    width: 'full',
    maxWidth: '[720px]',
    height: 'auto',
    marginTop: '24px',
    borderRadius: '[12px]',
    borderWidth: '1px',
    borderColor: 'border.hairline',
  });
  const bodyClass = css({ maxWidth: '[680px]', marginTop: '24px', fontSize: { base: '15px', md: '16px' }, color: 'text.default' });
</script>

<div class={listClass}>
  {#if feed.entries.length > 1}
    <div class={lineClass}></div>
  {/if}
  {#each feed.entries as entry (entry.id)}
    <article class={entryClass}>
      <div class={dateColClass}>
        <span class={dotClass}></span>
        <time class={dateClass} datetime={entry.date}>{formatDate(entry.date)}</time>
      </div>
      <div class={css({ minWidth: '0' })}>
        <h2 class={titleClass}>{entry.title}</h2>
        {#if entry.image}
          <img class={imageClass} alt={entry.title} loading="lazy" src={entry.image.url} />
        {/if}
        <div class={bodyClass}><Markdown blocks={parseMarkdown(entry.body)} /></div>
      </div>
    </article>
  {/each}
  <Sentinel {feed} />
</div>
