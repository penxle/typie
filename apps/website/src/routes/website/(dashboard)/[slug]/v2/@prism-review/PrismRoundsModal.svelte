<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal, TimeAgo } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import ReviewLensIcon from '~icons/typie/review-lens';
  import { requestSessionJump } from '$lib/prism/session-jump.svelte';
  import { getMarginContext } from './context.svelte.ts';
  import { groupRoundsByLineage } from './margin-view.ts';

  type Props = { open: boolean };
  let { open = $bindable() }: Props = $props();

  const margin = getMarginContext();

  // 계보가 하나뿐이면 무리 짓는 머리가 아무것도 가르지 않는다 — 둘 이상일 때만 세운다
  const groups = $derived(groupRoundsByLineage(margin.rounds));
  const grouped = $derived(groups.length > 1);

  const show = (roundId: string) => {
    mixpanel.track('open_prism_review_margin', { via: 'review_rounds_modal' });
    margin.select(roundId);
    open = false;
  };

  const goSession = (sessionId: string) => {
    requestSessionJump(sessionId);
    open = false;
  };

  const hide = () => {
    margin.select(null);
    open = false;
  };

  // 행 전체가 "에디터에 표시"다 — 접근성 조작면은 제목 버튼이고, 이 핸들러는 포인터 편의다.
  // 안쪽 버튼(대화 보기)에서 시작한 클릭은 제외한다
  const showFromRow = (event: MouseEvent, roundId: string) => {
    if (event.target instanceof Element && event.target.closest('button')) return;
    show(roundId);
  };

  // 표시 중 여부는 배지 글자가 말한다 — 보더 색은 그 배지를 거드는 부표지다
  const rowRecipe = cva({
    base: {
      borderWidth: '1px',
      borderRadius: '8px',
      padding: '12px',
      cursor: 'pointer',
      transition: '[border-color 0.15s ease, background-color 0.15s ease]',
      _hover: { backgroundColor: 'surface.subtle' },
    },
    variants: {
      shown: {
        true: { borderColor: 'border.brand' },
        false: { borderColor: 'border.subtle' },
      },
    },
  });
</script>

<Modal style={css.raw({ maxWidth: '420px' })} bind:open>
  <div class={center({ gap: '4px', padding: '12px' })}>
    <Icon style={css.raw({ color: 'text.faint' })} icon={ReviewLensIcon} size={14} />
    <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint' })}>프리즘 리뷰</span>
  </div>

  <HorizontalDivider />

  <div class={flex({ flexDirection: 'column', gap: '8px', padding: '16px' })}>
    {#each groups as group, index (group.lineageId)}
      {#if grouped}
        <!-- 첫 무리 위는 모달 머리의 구분선이 이미 긋는다 -->
        {#if index > 0}
          <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />
        {/if}
        <div class={css({ fontSize: '11px', color: 'text.faint' })}>
          {group.tierLabel} · 시작 {dayjs(group.startedAt).format('M월 D일')}
        </div>
      {/if}

      {#each group.rounds as round (round.id)}
        {@const shown = margin.selectedRoundId === round.id}
        {@const sessionId = round.sessionId}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class={css(rowRecipe.raw({ shown }))}
          aria-current={shown ? 'true' : undefined}
          onclick={(event) => showFromRow(event, round.id)}
        >
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <button
              class={flex({ alignItems: 'baseline', gap: '6px', flexGrow: '1', minWidth: '0', textAlign: 'left', cursor: 'pointer' })}
              aria-label={`${round.ordinal}회차 에디터에 표시`}
              onclick={() => show(round.id)}
              type="button"
            >
              <span class={css({ flex: 'none', fontSize: '13px', fontWeight: 'bold' })}>{round.ordinal}회차</span>
              <span class={css({ minWidth: '0', fontSize: '12px', color: 'text.subtle', truncate: true })}>{round.tierLabel}</span>
            </button>
            {#if shown}
              <span
                class={css({
                  flex: 'none',
                  paddingX: '6px',
                  paddingY: '2px',
                  borderRadius: '4px',
                  backgroundColor: 'accent.brand.subtle',
                  fontSize: '10px',
                  fontWeight: 'bold',
                  color: 'text.brand',
                })}
              >
                표시 중
              </span>
            {/if}
          </div>

          <div class={flex({ alignItems: 'center', gap: '12px', marginTop: '4px' })}>
            <span class={css({ minWidth: '0', fontSize: '11px', color: 'text.faint' })}>
              피드백 {round.issueCount}개 · <TimeAgo timestamp={new Date(round.createdAt).getTime()} />
            </span>
            {#if sessionId !== null}
              <button
                class={css({
                  flex: 'none',
                  marginLeft: 'auto',
                  fontSize: '11px',
                  color: 'text.faint',
                  cursor: 'pointer',
                  _hover: { color: 'text.subtle' },
                })}
                aria-label={`${round.ordinal}회차 대화 보기`}
                onclick={() => goSession(sessionId)}
                type="button"
              >
                대화 보기
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/each}
  </div>

  {#if margin.selectedRoundId !== null}
    <div class={flex({ paddingX: '16px', paddingBottom: '16px' })}>
      <Button onclick={hide} size="sm" variant="secondary">본문 표시 숨기기</Button>
    </div>
  {/if}
</Modal>
