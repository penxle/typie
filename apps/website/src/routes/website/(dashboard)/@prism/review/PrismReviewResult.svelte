<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import MessageSquareTextIcon from '~icons/lucide/message-square-text';
  import RepeatIcon from '~icons/lucide/repeat';
  import SparklesIcon from '~icons/lucide/sparkles';
  import TargetIcon from '~icons/lucide/target';
  import ThumbsDownIcon from '~icons/lucide/thumbs-down';
  import ThumbsUpIcon from '~icons/lucide/thumbs-up';
  import { goto } from '$app/navigation';
  import { requestMarginJump } from '$lib/prism/margin-jump.svelte';
  import { graphql } from '$mearie';
  import { expand } from '../lib/motion.ts';
  import PrismReactionNote from '../PrismReactionNote.svelte';
  import PrismReviewDetail from './PrismReviewDetail.svelte';
  import type { Component } from 'svelte';
  import type { ReviewRound } from './round-view.ts';

  type Props = { round: ReviewRound };

  let { round }: Props = $props();

  const [reactPrismReviewRound] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismReviewResult_React_Mutation($input: ReactPrismReviewRoundInput!) {
        reactPrismReviewRound(input: $input) {
          id
          reaction
          reactionNote
        }
      }
    `),
  );

  const reaction = $derived(round.reaction ?? null);
  const conclusion = $derived(round.conclusion ?? null);

  // 카드의 서사는 총평 서두에서 빌린다 — 재리뷰면 진전(progress)이 이번 회차의 이야기라 이해보다 앞선다.
  // 첫 문단만 세운다: 전문은 총평 모달의 몫이고, 여기는 지나치지 않게 붙드는 맛보기다
  const lead = $derived.by(() => {
    const text = conclusion?.progress ?? conclusion?.understanding ?? null;
    if (!text) return null;
    return (
      text
        .split('\n')
        .map((line) => line.trim())
        .find((line) => line.length > 0) ?? null
    );
  });

  let editing = $state(false);
  let note = $state('');
  let sending = $state(false);
  let detailOpen = $state(false);

  const send = async (roundId: string, value: 'DOWN' | 'UP' | null, text: string | null) => {
    if (sending) {
      return false;
    }

    sending = true;

    try {
      await reactPrismReviewRound({ input: { roundId, value: value ?? undefined, note: text ?? undefined } });
      return true;
    } catch {
      Toast.error('반응을 남기지 못했어요');
      return false;
    } finally {
      sending = false;
    }
  };

  const react = async (value: 'DOWN' | 'UP') => {
    const draft = note.trim();
    const kept = draft.length > 0 ? draft : (round.reactionNote ?? null);
    const clearing = (round.reaction ?? null) === value;
    const ok = clearing ? await send(round.id, null, null) : await send(round.id, value, kept);

    if (ok) {
      if (clearing) {
        note = '';
        editing = false;
      } else {
        editing = kept === null;
      }
    }
  };

  const saveNote = async () => {
    const current = round.reaction ?? null;
    if (current === null) {
      return;
    }

    if (await send(round.id, current, note.trim() || null)) {
      editing = false;
    }
  };

  const openMargin = async () => {
    mixpanel.track('open_prism_review_margin', { via: 'review_result' });
    requestMarginJump({ documentId: round.document.id, roundId: round.id, itemId: null });
    await goto(`/${round.document.entity.slug}`);
  };

  // 시작 확인 카드와 같은 카드 문법 — 카드 겉·13px 기준 서체·민제목·보더 서브행·간격까지 그대로 따른다
  const cardClass = css({
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '10px',
    padding: '14px',
    fontSize: '13px',
    backgroundColor: 'surface.default',
    boxShadow: 'sm',
  });
  const titleClass = css({ fontSize: '13px', fontWeight: 'semibold', marginBottom: '10px' });
  // 수치는 서브행 상자 안에 항목별 행으로 담는다 — 행마다 아이콘·라벨·값의 같은 구조를 반복한다
  const countsBoxClass = flex({
    flexDirection: 'column',
    gap: '8px',
    paddingX: '10px',
    paddingY: '10px',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '8px',
  });

  const thumbStyle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    size: '26px',
    borderWidth: '1px',
    borderRadius: '6px',
    flexShrink: '0',
    transition: '[background-color 150ms ease, border-color 150ms ease]',
  });
  const thumbOnStyle = css.raw({ borderColor: 'accent.default', backgroundColor: 'surface.active', color: 'text.default' });
  const thumbOffStyle = css.raw({
    borderColor: 'border.default',
    backgroundColor: 'surface.default',
    color: 'text.muted',
    _hover: { color: 'text.default' },
  });
</script>

{#snippet statRow(icon: Component, label: string, count: number)}
  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <Icon style={css.raw({ color: 'text.muted' })} {icon} size={14} />
    <span class={css({ color: 'text.muted' })}>{label}</span>
    <span class={css({ fontWeight: 'semibold' })}>{count}개</span>
  </div>
{/snippet}

<div class={cardClass}>
  <div class={titleClass}>리뷰를 마쳤어요</div>

  {#if lead !== null}
    <p class={css({ marginBottom: '10px', lineHeight: '[1.65]', color: 'text.muted', lineClamp: '4' })}>{lead}</p>
  {/if}

  <div class={countsBoxClass}>
    {@render statRow(MessageSquareTextIcon, '피드백', round.issueCount)}
    {#if conclusion !== null}
      {#if conclusion.strengthsCount > 0}
        {@render statRow(SparklesIcon, '강점', conclusion.strengthsCount)}
      {/if}
      {@render statRow(RepeatIcon, '패턴', conclusion.patternsCount)}
      {@render statRow(TargetIcon, '우선순위', conclusion.prioritiesCount)}
    {/if}
  </div>

  {#if round.dispositionSummary}
    {@const s = round.dispositionSummary}
    <p class={css({ marginTop: '8px', fontSize: '11px', color: 'text.hint' })}>
      이어진 {s.carried} · 해소 {s.resolved} · 철회 {s.withdrawn} · 신규 {s.new}
    </p>
  {/if}

  <div class={flex({ flexDirection: 'column', gap: '8px', marginTop: '12px' })}>
    {#if round.hasDetail}
      <Button
        style={css.raw({ width: 'full' })}
        onclick={() => {
          detailOpen = true;
          mixpanel.track('open_prism_review_conclusion');
        }}
        size="sm"
        variant="secondary"
      >
        총평 읽기
      </Button>
    {/if}
    <Button style={css.raw({ width: 'full' })} onclick={() => void openMargin()} size="sm" variant="secondary">본문에 표시하기</Button>
  </div>

  <div
    class={flex({
      alignItems: 'center',
      gap: '6px',
      marginTop: '12px',
      paddingTop: '10px',
      borderTopWidth: '1px',
      borderColor: 'border.hairline',
    })}
  >
    <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '11px', color: 'text.muted' })}>이번 리뷰 어땠나요?</span>
    <button
      class={css(thumbStyle, reaction === 'UP' ? thumbOnStyle : thumbOffStyle)}
      aria-label="좋았어요"
      aria-pressed={reaction === 'UP'}
      disabled={sending}
      onclick={() => void react('UP')}
      type="button"
    >
      <Icon icon={ThumbsUpIcon} size={12} />
    </button>
    <button
      class={css(thumbStyle, reaction === 'DOWN' ? thumbOnStyle : thumbOffStyle)}
      aria-label="아쉬웠어요"
      aria-pressed={reaction === 'DOWN'}
      disabled={sending}
      onclick={() => void react('DOWN')}
      type="button"
    >
      <Icon icon={ThumbsDownIcon} size={12} />
    </button>
  </div>

  {#if reaction !== null}
    <div class={css({ marginTop: '10px' })} transition:expand>
      <PrismReactionNote onSubmit={() => void saveNote()} savedNote={round.reactionNote ?? null} {sending} bind:editing bind:draft={note} />
    </div>
  {/if}
</div>

{#if round.hasDetail}
  <PrismReviewDetail {round} bind:open={detailOpen} />
{/if}
