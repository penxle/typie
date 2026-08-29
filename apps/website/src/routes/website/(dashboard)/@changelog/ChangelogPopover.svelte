<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import XIcon from '~icons/lucide/x';
  import { fetchHighlight, seenIdFor, shouldShowPopover } from './changelog-state.svelte';
  import type { ChangelogHighlight } from './changelog-state.svelte';

  type Props = {
    suppressed: boolean;
  };

  let { suppressed }: Props = $props();

  const app = getAppContext();

  let highlight = $state<ChangelogHighlight | null>(null);
  let dismissed = $state(false);
  let imageBroken = $state(false);
  let imageLoaded = $state(false);
  let imageInstant = $state(false);
  let tracked = $state('');

  const revealImage = (node: HTMLImageElement) => {
    if (node.complete && node.naturalWidth > 0) {
      imageInstant = true;
      imageLoaded = true;
    }
  };

  const reduceMotion = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const enterMotion = reduceMotion ? { y: 0, duration: 0 } : { y: 8, duration: 200 };
  const leaveMotion = { duration: reduceMotion ? 0 : 100 };

  $effect(() => {
    void (async () => {
      const entry = await fetchHighlight();
      highlight = entry;

      untrack(() => {
        if (app.preference.current.changelogSeenId === '') {
          app.preference.current.changelogSeenId = seenIdFor(entry);
        }
      });
    })();
  });

  const visible = $derived(!suppressed && !dismissed && shouldShowPopover(highlight, app.preference.current.changelogSeenId));

  $effect(() => {
    if (!visible || !highlight) return;

    const id = highlight.id;
    untrack(() => {
      if (tracked !== id) {
        tracked = id;
        mixpanel.track('view_changelog_popover', { changelogId: id });
      }
    });
  });

  const markSeen = () => {
    app.preference.current.changelogSeenId = seenIdFor(highlight);
    dismissed = true;
  };

  $effect(() => {
    if (!app.state.changelogOpen) return;

    const entry = highlight;

    untrack(() => {
      if (entry && !dismissed && shouldShowPopover(entry, app.preference.current.changelogSeenId)) {
        markSeen();
      }
    });
  });

  const open = () => {
    mixpanel.track('open_changelog_modal', { via: 'popover' });
    markSeen();
    app.state.changelogOpen = true;
  };

  const dismiss = () => {
    mixpanel.track('dismiss_changelog_popover', { changelogId: highlight?.id });
    markSeen();
  };
</script>

{#if visible && highlight}
  <div
    class={css({
      position: 'absolute',
      bottom: '54px',
      left: '0',
      right: '0',
      paddingX: '12px',
      paddingTop: '12px',
      pointerEvents: 'none',
    })}
    in:fly={enterMotion}
    out:fade={leaveMotion}
  >
    <div class={cx('group', css({ position: 'relative', pointerEvents: 'auto' }))}>
      <button
        class={css({
          display: 'block',
          width: 'full',
          padding: '14px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          backgroundColor: 'surface.default',
          boxShadow: 'medium',
          textAlign: 'left',
          cursor: 'pointer',
          transitionProperty: '[border-color, box-shadow]',
          transitionDuration: '200ms',
          transitionTimingFunction: 'ease',
          _hover: { borderColor: 'border.strong' },
        })}
        onclick={open}
        type="button"
      >
        {#if highlight.image && !imageBroken}
          <img
            class={css({
              width: 'full',
              aspectRatio: '[16/9]',
              objectFit: 'cover',
              borderRadius: '8px',
              marginBottom: '12px',
              backgroundColor: 'surface.muted',
              opacity: imageLoaded ? '100' : '0',
              transitionProperty: '[opacity]',
              transitionDuration: imageInstant ? '0ms' : '300ms',
              transitionTimingFunction: 'ease',
            })}
            {@attach revealImage}
            alt=""
            loading="lazy"
            onerror={() => (imageBroken = true)}
            onload={() => (imageLoaded = true)}
            src={highlight.image.url}
          />
        {/if}

        <div class={css({ marginBottom: '6px', fontSize: '11px', color: 'text.faint' })}>
          {dayjs(highlight.date).formatAsDate()} · 업데이트 노트
        </div>

        <div
          class={css({
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'text.default',
            lineHeight: '[1.5]',
            lineClamp: '2',
          })}
        >
          {highlight.title}
        </div>
      </button>

      <button
        class={css({
          position: 'absolute',
          top: '8px',
          right: '8px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          size: '20px',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '6px',
          color: 'text.faint',
          backgroundColor: 'surface.default',
          cursor: 'pointer',
          transitionProperty: '[opacity, background-color, border-color, color]',
          transitionDuration: '200ms',
          transitionTimingFunction: 'ease',
          opacity: '0',
          _groupHover: { opacity: '100' },
          _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
          _focusVisible: { opacity: '100', backgroundColor: 'surface.muted', color: 'text.subtle' },
        })}
        aria-label="닫기"
        onclick={dismiss}
        type="button"
      >
        <Icon icon={XIcon} size={12} />
      </button>
    </div>
  </div>
{/if}
