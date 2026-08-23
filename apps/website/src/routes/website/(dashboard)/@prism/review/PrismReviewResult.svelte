<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import ThumbsDownIcon from '~icons/lucide/thumbs-down';
  import ThumbsUpIcon from '~icons/lucide/thumbs-up';
  import { goto } from '$app/navigation';
  import { requestMarginJump } from '$lib/prism/margin-jump.svelte';
  import { graphql } from '$mearie';
  import { expand, swap } from '../lib/motion.ts';
  import PrismReviewDetail from './PrismReviewDetail.svelte';
  import type { ResultView, ReviewRound } from './round-view.ts';

  type Props = { round: ReviewRound; result: Exclude<ResultView, { kind: 'rejected' }> };

  let { round, result }: Props = $props();

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

  let editing = $state(false);
  let note = $state('');
  let sending = $state(false);
  let detailOpen = $state(false);

  let noteEl = $state<HTMLElement>();
  let noteFrom = $state<number>();
  let prevNoteMode: string | undefined;

  $effect.pre(() => {
    const mode = !editing && round.reactionNote ? 'saved' : 'draft';
    if (prevNoteMode !== undefined && mode !== prevNoteMode) noteFrom = noteEl?.offsetHeight;
    prevNoteMode = mode;
  });

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

    if (ok && clearing) {
      note = '';
      editing = false;
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
    requestMarginJump({ documentId: round.document.id, roundId: round.id, itemId: null });
    await goto(`/${round.document.entity.slug}`);
  };

  // 누를 수 있는 곳은 글자만큼이다 — 블록 폭을 그대로 쓰면 오른쪽 빈 자리까지 반응한다
  const lineClass = css({
    display: 'block',
    width: '[fit-content]',
    fontSize: '[12.5px]',
    textAlign: 'left',
    color: 'text.faint',
    _hover: { color: 'text.subtle' },
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
  const thumbOnStyle = css.raw({ borderColor: 'border.strong', backgroundColor: 'surface.muted', color: 'text.default' });
  const thumbOffStyle = css.raw({
    borderColor: 'border.default',
    backgroundColor: 'surface.default',
    color: 'text.faint',
    _hover: { color: 'text.subtle' },
  });
</script>

<div>
  <button class={lineClass} onclick={() => void openMargin()} type="button">
    {result.issues}{#if result.kind === 'summary'}
      · {result.counts}{/if}
  </button>

  {#if round.hasDetail}
    <div>
      <button
        class={css({
          marginTop: '10px',
          fontSize: '12px',
          fontWeight: 'semibold',
          color: 'text.subtle',
          _hover: { color: 'text.default' },
        })}
        onclick={() => (detailOpen = true)}
        type="button"
      >
        총평 읽기
      </button>
    </div>

    <PrismReviewDetail {round} bind:open={detailOpen} />
  {/if}

  <div class={flex({ alignItems: 'center', gap: '6px', marginTop: '10px' })}>
    <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '11px', color: 'text.faint' })}>이번 리뷰 어땠나요?</span>
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
      <div bind:this={noteEl}>
        {#if round.reactionNote && !editing}
          {@const saved = round.reactionNote}
          <div class={flex({ alignItems: 'center', gap: '8px' })} in:swap={{ box: noteEl, from: noteFrom }}>
            <p
              class={css({
                flexGrow: '1',
                minWidth: '0',
                fontSize: '12px',
                lineHeight: '[1.55]',
                color: 'text.subtle',
                whiteSpace: 'pre-wrap',
              })}
            >
              {saved}
            </p>
            <button
              class={css({
                flexShrink: '0',
                fontSize: '11px',
                fontWeight: 'semibold',
                color: 'text.subtle',
                _hover: { color: 'text.default' },
              })}
              onclick={() => {
                note = saved;
                editing = true;
              }}
              type="button"
            >
              수정
            </button>
          </div>
        {:else}
          <div
            class={flex({
              alignItems: 'flex-end',
              gap: '6px',
              paddingLeft: '10px',
              paddingRight: '4px',
              paddingY: '4px',
              borderWidth: '1px',
              borderColor: 'border.default',
              borderRadius: '8px',
              backgroundColor: 'surface.subtle',
            })}
            in:swap={{ box: noteEl, from: noteFrom }}
          >
            <textarea
              class={css({
                flexGrow: '1',
                minWidth: '0',
                maxHeight: '120px',
                paddingY: '3px',
                fontSize: '12px',
                lineHeight: '[1.5]',
                backgroundColor: 'transparent',
                resize: 'none',
                outline: 'none',
                _placeholder: { color: 'text.faint' },
              })}
              onkeydown={(event) => {
                if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
                  event.preventDefault();
                  void saveNote();
                }
              }}
              placeholder="몇 자 덧붙이기"
              rows={1}
              bind:value={note}
              use:autosize={{ value: note }}></textarea>
            <button
              class={flex({
                alignItems: 'center',
                justifyContent: 'center',
                flexShrink: '0',
                size: '22px',
                borderRadius: 'full',
                backgroundColor: 'surface.dark',
                color: 'text.bright',
              })}
              aria-label="남기기"
              disabled={sending}
              onclick={() => void saveNote()}
              type="button"
            >
              <Icon icon={ArrowUpIcon} size={10} />
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
