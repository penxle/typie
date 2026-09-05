<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';
  import NewspaperIcon from '~icons/lucide/newspaper';
  import XIcon from '~icons/lucide/x';
  import { parseMarkdown } from '$lib/markdown/parse';
  import { fetchPage } from './changelog-state.svelte';
  import ChangelogMarkdown from './ChangelogMarkdown.svelte';
  import type { ChangelogEntry } from './changelog-state.svelte';

  const app = getAppContext();

  let entries = $state<ChangelogEntry[]>([]);
  let nextPage = $state(1);
  let hasMore = $state(true);
  let loading = $state(false);
  let failed = $state(false);
  let sentinel = $state<HTMLDivElement | null>(null);
  let scrollEl = $state<HTMLDivElement | null>(null);

  const loadMore = async () => {
    if (loading || !hasMore) return;

    loading = true;

    try {
      const page = await fetchPage(nextPage);

      if (page === null) {
        if (entries.length === 0) {
          failed = true;
        }

        return;
      }

      const seen = new Set(entries.map((entry) => entry.id));

      entries = [...entries, ...page.entries.filter((entry) => !seen.has(entry.id))];
      hasMore = page.hasMore;
      nextPage += 1;
    } finally {
      loading = false;
    }
  };

  $effect(() => {
    if (!app.state.changelogOpen) return;

    untrack(() => {
      failed = false;

      if (!loading && entries.length === 0) {
        void loadMore();
      }
    });
  });

  $effect(() => {
    const target = sentinel;
    const root = scrollEl;
    if (!target || !root) return;

    const observer = new IntersectionObserver(
      (records) => {
        if (records.some((record) => record.isIntersecting)) {
          void loadMore();
        }
      },
      { root, rootMargin: '0px 0px 300px 0px' },
    );

    observer.observe(target);

    return () => observer.disconnect();
  });
</script>

<Modal
  style={css.raw({ gap: '0', maxWidth: '600px', paddingX: '24px', paddingTop: '12px', paddingBottom: '24px' })}
  onclose={() => {
    app.state.changelogOpen = false;
  }}
  open={app.state.changelogOpen}
>
  <div
    class={css({
      display: 'grid',
      gridTemplateColumns: '[1fr auto 1fr]',
      alignItems: 'center',
      marginX: '-24px',
      paddingX: '16px',
      paddingBottom: '10px',
      borderBottomWidth: '1px',
      borderColor: 'border.hairline',
      fontSize: '14px',
      fontWeight: 'medium',
      lineHeight: '[1.4]',
      color: 'text.muted',
    })}
  >
    <div></div>

    <div class={flex({ alignItems: 'center', justifyContent: 'center', gap: '6px' })}>
      <Icon style={css.raw({ flexShrink: '0' })} icon={NewspaperIcon} size={14} />
      업데이트 노트
    </div>

    <div class={flex({ justifyContent: 'flex-end' })}>
      <button
        class={css({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          marginY: '-4px',
          padding: '4px',
          borderRadius: '6px',
          color: 'text.muted',
          cursor: 'pointer',
          transitionProperty: '[background-color, color]',
          transitionDuration: '200ms',
          transitionTimingFunction: 'ease',
          _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
        })}
        aria-label="닫기"
        onclick={() => {
          app.state.changelogOpen = false;
        }}
        type="button"
      >
        <Icon icon={XIcon} size={16} />
      </button>
    </div>
  </div>

  {#if failed}
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'center',
        height: '[min(80vh, 800px)]',
        marginBottom: '-24px',
        fontSize: '14px',
        color: 'text.hint',
      })}
    >
      업데이트 노트를 불러오지 못했어요
    </div>
  {:else}
    <div
      bind:this={scrollEl}
      class={flex({
        direction: 'column',
        height: '[min(80vh, 800px)]',
        marginX: '-24px',
        marginBottom: '-24px',
        paddingX: '24px',
        paddingTop: '12px',
        paddingBottom: '24px',
        overflowY: 'auto',
      })}
    >
      {#each entries as entry, idx (entry.id)}
        <article
          class={css({
            position: 'relative',
            display: 'grid',
            gridTemplateColumns: '[100px minmax(0, 1fr)]',
            gap: '20px',
          })}
        >
          {#if entries.length > 1}
            <div
              style:top={idx === 0 ? '14px' : '0'}
              style:bottom={idx === entries.length - 1 ? 'auto' : '0'}
              style:height={idx === entries.length - 1 ? '14px' : undefined}
              class={css({ position: 'absolute', left: '3px', width: '1px', backgroundColor: 'border.hairline' })}
            ></div>
          {/if}

          <div>
            <div
              class={flex({
                position: 'sticky',
                top: '0',
                alignItems: 'center',
                gap: '10px',
                height: '31px',
                paddingTop: '5px',
                paddingBottom: '8px',
              })}
            >
              <div class={css({ flexShrink: '0', size: '7px', borderRadius: 'full', backgroundColor: 'border.emphasis' })}></div>
              <span
                class={css({
                  fontSize: '12px',
                  fontWeight: 'medium',
                  color: 'text.muted',
                  whiteSpace: 'nowrap',
                  fontVariantNumeric: 'tabular-nums',
                })}
              >
                {dayjs(entry.date).formatAsDate()}
              </span>
            </div>
          </div>

          <div style:padding-bottom={idx === entries.length - 1 ? undefined : '60px'} class={flex({ direction: 'column' })}>
            <h2 class={css({ marginBottom: '16px', fontSize: '20px', fontWeight: 'bold', color: 'text.default', lineHeight: '[1.35]' })}>
              {entry.title}
            </h2>

            {#if entry.image}
              <img
                class={css({
                  width: 'full',
                  height: 'auto',
                  marginBottom: '22px',
                  borderRadius: '8px',
                  borderWidth: '1px',
                  borderColor: 'border.hairline',
                })}
                alt={entry.title}
                loading="lazy"
                src={entry.image.url}
              />
            {/if}

            <div class={css({ fontSize: '15px', color: 'text.default' })}>
              <ChangelogMarkdown blocks={parseMarkdown(entry.body)} />
            </div>
          </div>
        </article>
      {/each}

      {#if hasMore}
        {#key nextPage}
          <div bind:this={sentinel} class={css({ paddingY: '12px' })}></div>
        {/key}
      {/if}
    </div>
  {/if}
</Modal>
