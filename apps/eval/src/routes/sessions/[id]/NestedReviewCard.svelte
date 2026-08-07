<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import { capsuleLabel } from '$lib/feedback/live.ts';
  import type { Snippet } from 'svelte';
  import type { FeedEntry } from '$lib/feedback/live.ts';

  // 계획 왕복은 스테이지를 넘지 않는다 — 바깥 카드와 같은 문법(제목 줄 + 발화·캡슐 기록 + 개폐)을 축소해 한 겹
  // 안에 둔다. 통과·보완 같은 판정은 여기서 추론하지 않는다 — 화면에 서는 것은 모델의 발화와 활동 자취뿐이다.
  // live는 바깥에서 그리던 라이브 줄+활동 행을, ghost는 봉인 드레이너의 꼬리 줄을 그대로 넘겨받는 자리다.
  type Props = {
    round: number;
    feed: FeedEntry[];
    spent?: string | null;
    latest?: boolean;
    live?: Snippet | null;
    ghost?: Snippet | null;
  };

  const { round, feed, spent = null, latest = false, live = null, ghost = null }: Props = $props();

  let expanded = $state(false);

  const lines = $derived(feed.filter((entry): entry is Extract<FeedEntry, { kind: 'line' }> => entry.kind === 'line'));

  // 돌고 있는 동안(드레인 포함)에는 전량 흐르고, 끝나면 마지막 발화 한 줄만 영수증으로 남는다(펼치면 캡슐까지 전량).
  const active = $derived(live !== null || ghost !== null);
  const shown = $derived(!active && !expanded ? lines.slice(-1) : feed);
  const toggleable = $derived(!active && (feed.length > 1 || (feed.length === 1 && feed[0].kind !== 'line')));

  // 발화 없이 지나간 라운드 — 없는 말을 지어내지 않고 걸린 시간만 영수증으로 세운다.
  const silent = $derived(!active && lines.length === 0 ? spent : null);

  const lastLineId = $derived(lines.at(-1)?.line.id ?? null);

  const cardRecipe = cva({
    base: {
      marginY: '5px',
      borderWidth: '1px',
      borderRadius: '8px',
      transition: '[border-color 0.4s ease, background-color 0.4s ease]',
    },
    variants: {
      live: {
        true: { borderColor: 'border.default', backgroundColor: 'surface.default' },
        false: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
      },
    },
  });

  const headerRecipe = cva({
    base: { display: 'flex', alignItems: 'center', gap: '6px', width: 'full', paddingX: '9px', paddingY: '6px', textAlign: 'left' },
    variants: {
      clickable: {
        true: { cursor: 'pointer' },
        false: {},
      },
    },
  });

  const dotRecipe = cva({
    base: { size: '5px', flex: 'none', borderRadius: 'full', transition: '[background-color 0.4s ease]' },
    variants: {
      live: {
        true: { backgroundColor: 'accent.brand.default', animation: 'breathe 2s ease-in-out infinite' },
        false: { backgroundColor: 'border.strong' },
      },
    },
  });

  const titleRecipe = cva({
    base: { flex: 'none', fontSize: '11px', fontWeight: 'semibold', transition: '[color 0.3s ease]' },
    variants: {
      live: {
        true: { color: 'text.default' },
        false: { color: 'text.muted' },
      },
    },
  });

  const bodyClass = css({ paddingX: '9px', paddingBottom: '7px' });

  // 바깥 피드의 캡슐과 같은 급(11px·disabled) — 카드 안이라고 말이 달라지지 않는다.
  const capsuleClass = css({ paddingY: '2px', fontSize: '11px', lineHeight: '[1.7]', color: 'text.disabled' });

  // 바깥 피드 줄과 같은 눈금(12px)을 쓴다 — 라이브 줄이 이 카드 안으로 들어와도 글줄이 어긋나지 않게.
  const lineRecipe = cva({
    base: {
      paddingY: '2px',
      fontSize: '12px',
      lineHeight: '[1.6]',
      whiteSpace: 'pre-wrap',
      transition: '[color 0.4s ease, font-weight 0.4s ease]',
    },
    variants: {
      latest: {
        true: { fontWeight: 'semibold', color: 'text.default' },
        false: { color: 'text.faint' },
      },
    },
  });
</script>

<div class={css(cardRecipe.raw({ live: active }))}>
  <button
    class={css(headerRecipe.raw({ clickable: toggleable }))}
    aria-expanded={toggleable ? expanded : undefined}
    disabled={!toggleable}
    onclick={() => (expanded = !expanded)}
    type="button"
  >
    <span class={css(dotRecipe.raw({ live: active }))}></span>

    <span class={css(titleRecipe.raw({ live: active }))}>계획 점검 {round}</span>

    {#if toggleable}
      <Icon
        style={css.raw({ flex: 'none', marginLeft: 'auto', color: 'text.disabled' })}
        icon={expanded ? IconChevronUp : IconChevronDown}
        size={10}
      />
    {:else if silent}
      <span class={css({ marginLeft: 'auto', flex: 'none', fontSize: '10px', color: 'text.disabled' })}>{silent}</span>
    {/if}
  </button>

  {#if shown.length > 0 || active}
    <div class={bodyClass}>
      {#each shown as entry (entry.key)}
        {#if entry.kind === 'line'}
          <div class={css(lineRecipe.raw({ latest: latest && entry.line.id === lastLineId }))}>{entry.line.text}</div>
        {:else}
          <div class={capsuleClass}>{entry.items.map(capsuleLabel).join(' · ')}</div>
        {/if}
      {/each}

      {#if ghost !== null}
        {@render ghost()}
      {/if}
      {#if live !== null}
        {@render live()}
      {/if}
    </div>
  {/if}
</div>
