<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import SmileIcon from '~icons/lucide/smile';
  import type { Snippet } from 'svelte';

  type Props = {
    href?: string | null;
    title: string;
    titleHtml?: string | null;
    subtitle?: string | null;
    excerpt?: string | null;
    excerptHtml?: string | null;
    tags: readonly string[];
    tagHref?: ((tag: string) => string) | null;
    hasPassword?: boolean;
    passwordUnlocked?: boolean;
    timestamp?: number | null;
    reactionCount?: number;
    enter?: boolean;
    folders?: readonly { name: string; href?: string | null }[];
    lead?: Snippet;
    thumbnail?: Snippet;
  };

  let {
    href = null,
    title,
    titleHtml = null,
    subtitle = null,
    excerpt = null,
    excerptHtml = null,
    tags,
    tagHref = null,
    hasPassword = false,
    passwordUnlocked = false,
    timestamp = null,
    reactionCount = 0,
    enter = false,
    folders = [],
    lead,
    thumbnail,
  }: Props = $props();

  const hasLead = $derived(!!lead || folders.length > 0);
  const hasMeta = $derived(hasLead || timestamp !== null);
  const hasExcerpt = $derived(!!(excerptHtml || excerpt));

  const meta = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    minWidth: '0',
    fontSize: '12px',
    color: 'text.hint',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  });

  const dot = css.raw({ flexShrink: '0', size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' });
  const highlight = css.raw({ '& em': { fontStyle: 'normal', color: 'accent.default' } });
  const tagStyle = css.raw({ flexShrink: '0', transition: 'colors', _hover: { color: 'text.default' } });
  const folderStyle = css.raw({
    minWidth: '0',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    fontWeight: 'medium',
    color: 'text.muted',
    transition: 'colors',
    _hover: { color: 'text.default' },
  });
</script>

<article
  class={css({
    minWidth: '0',
    paddingY: '24px',
    borderTopWidth: '1px',
    borderColor: 'border.hairline',
    _first: { paddingTop: '4px', borderTopWidth: '0' },
  })}
  in:fly={enter ? { y: 6, duration: 220, easing: cubicOut } : { duration: 0 }}
>
  <div class={css({ minWidth: '0' })}>
    {#if hasMeta}
      <div class={css(meta, { height: '20px', marginBottom: '8px' })}>
        {#if lead}
          {@render lead()}
        {:else if folders.length > 0}
          <span class={flex({ alignItems: 'center', gap: '2px', minWidth: '0' })}>
            {#each folders as folder, index (index)}
              {#if index > 0}
                <Icon style={css.raw({ flexShrink: '0' })} icon={ChevronRightIcon} size={12} />
              {/if}
              {#if folder.href}
                <a class={css(folderStyle)} href={folder.href}>{folder.name}</a>
              {:else}
                <span class={css(folderStyle)}>{folder.name}</span>
              {/if}
            {/each}
          </span>
        {/if}
        {#if hasLead && timestamp !== null}
          <i class={css(dot)} aria-hidden="true"></i>
        {/if}
        {#if timestamp !== null}
          <TimeAgo {timestamp} />
        {/if}
      </div>
    {/if}

    <svelte:element
      this={href ? 'a' : 'div'}
      class={flex({
        alignItems: 'flex-start',
        gap: '14px',
        minWidth: '0',
        _hover: {
          '& [data-card-title]': { color: 'text.muted' },
          '& [data-card-cover]': { transform: 'scale(1.03)' },
        },
      })}
      href={href ?? undefined}
    >
      <div class={css({ flex: '1', minWidth: '0', maxWidth: '640px' })}>
        <h3
          class={flex({
            alignItems: 'flex-start',
            gap: '6px',
            fontSize: '17px',
            fontWeight: 'semibold',
            lineHeight: '[1.35]',
            letterSpacing: '-0.015em',
            '@media (max-width: 639px)': { fontSize: '16px' },
          })}
        >
          {#if hasPassword}
            <Icon
              style={css.raw({ flexShrink: '0', marginTop: '4px', color: 'text.muted' })}
              icon={passwordUnlocked ? LockOpenIcon : LockIcon}
              size={14}
            />
          {/if}
          <span class={css(highlight, { minWidth: '0', transition: 'colors', lineClamp: '2' })} data-card-title>
            {#if titleHtml}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html titleHtml}
            {:else}
              {title}
            {/if}
          </span>
        </h3>

        {#if subtitle}
          <h3
            class={css({
              marginTop: '3px',
              fontSize: '14px',
              fontWeight: 'medium',
              lineHeight: '[1.5]',
              color: 'text.muted',
              lineClamp: '1',
            })}
          >
            {subtitle}
          </h3>
        {/if}

        {#if hasExcerpt}
          <p
            class={css(highlight, {
              marginTop: '8px',
              fontFamily: 'prose',
              fontSize: '14px',
              lineHeight: '[1.6]',
              letterSpacing: '-0.005em',
              color: 'text.muted',
              lineClamp: '2',
            })}
          >
            {#if excerptHtml}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html excerptHtml}
            {:else}
              {excerpt}
            {/if}
          </p>
        {/if}
      </div>

      {#if thumbnail}
        <div
          class={css({
            flexShrink: '0',
            width: '128px',
            marginTop: '3px',
            marginLeft: 'auto',
            aspectRatio: '[16 / 9]',
            borderRadius: '4px',
            backgroundColor: 'surface.canvas',
            overflow: 'hidden',
            isolation: 'isolate',
            '@media (max-width: 639px)': { width: '104px' },
          })}
        >
          {@render thumbnail()}
        </div>
      {/if}
    </svelte:element>

    {#if tags.length > 0 || reactionCount > 0}
      <div class={css(meta, { alignItems: 'flex-start', marginTop: '12px', whiteSpace: 'normal' })}>
        {#if tags.length > 0}
          <span class={flex({ flexWrap: 'wrap', columnGap: '6px', rowGap: '4px', minWidth: '0' })}>
            {#each tags as tag (tag)}
              {#if tagHref}
                <a class={css(tagStyle)} href={tagHref(tag)}>#{tag}</a>
              {:else}
                <span class={css(tagStyle)}>#{tag}</span>
              {/if}
            {/each}
          </span>
        {/if}
        {#if reactionCount > 0}
          <span class={flex({ alignItems: 'center', gap: '3px', flexShrink: '0', marginLeft: 'auto' })}>
            <Icon icon={SmileIcon} size={14} />
            {comma(reactionCount)}
          </span>
        {/if}
      </div>
    {/if}
  </div>
</article>
