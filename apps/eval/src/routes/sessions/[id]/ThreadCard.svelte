<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import IconArrowUp from '~icons/lucide/arrow-up';
  import IconCircleCheck from '~icons/lucide/circle-check';
  import IconThumbsDown from '~icons/lucide/thumbs-down';
  import IconThumbsUp from '~icons/lucide/thumbs-up';
  import { enhance } from '$app/forms';
  import Paragraphs from './Paragraphs.svelte';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];
  type Comment = PageData['comments'][number];

  type Props = {
    thread: Thread;
    comments: Comment[];
    quote: string;
    pattern: { theme: string; count: number } | null;
    priority: { rank: number; total: number; body: string } | null;
    expanded: boolean;
    locked: boolean;
    // 표시 회차 — 재열기 가능 여부(이번 회차 닫힘)의 판정 축이다.
    round: number;
    // 마지막 리뷰에서 새로 생긴 피드백 — 이전 회차 카드가 존재하는 세션에서만 컬럼이 켠다(첫 리뷰는 전부
    // 신규라 뱃지가 소음이 된다).
    isNew?: boolean;
    onToggle: () => void;
  };

  const { thread, comments, quote, pattern, priority, expanded, locked, round, isNew = false, onToggle }: Props = $props();

  // 닫힘(작가가 접음)·해소·철회(재리뷰 처분)는 조작면과 흐림 처리를 공유한다 — 되돌리기는 닫힘만의 몫이다.
  const settled = $derived(thread.state !== 'open');
  const closed = $derived(thread.state === 'closed');
  // 다시 열기는 이번 회차에 닫은 카드만 — 과거 회차 닫힘의 재열기는 "이동은 재리뷰 시에만" 규칙 위반이고,
  // 옛 좌표 스레드가 열린 모드로 와 마크가 어긋난 자리에 그어진다.
  const reopenable = $derived(closed && thread.reviewRound === round);
  const snippet = $derived(thread.body ?? '');
  // 빈 본문 코멘트는 그리지 않는다 — 아바타와 시각만 남은 껍데기 줄이 대화에 끼어들면 안 된다.
  const shownComments = $derived(comments.filter((comment) => comment.body.trim().length > 0));
  const meta = $derived(shownComments.length > 0 ? `댓글 ${shownComments.length}` : '');

  const STATE_LABELS = { closed: '닫음', resolved: '해결됨', withdrawn: '거둠' };

  const authorLabel = (comment: Comment) => (comment.author === 'ai' ? 'AI' : '나');

  // 더블클릭이 댓글 2행·자기 유발 409를 만든다 — 왕복 중인 폼은 제출을 취소하고 조작면을 잠근다.
  let busy = $state<'reply' | 'close' | 'reopen' | 'delete' | 'react' | null>(null);

  // form.reset()은 input 이벤트를 쏘지 않아 autosize가 못 본다 — 값을 상태로 들고 성공 시 비워서 재계산을 태운다.
  let replyBody = $state('');

  const submit =
    (kind: 'reply' | 'close' | 'reopen' | 'delete' | 'react'): SubmitFunction =>
    ({ cancel }) => {
      if (busy !== null) {
        cancel();
        return;
      }
      busy = kind;
      // update()가 거부하면 잠금이 영구화된다 — 해제는 성패와 무관하게 한다.
      return async ({ result, update }) => {
        try {
          if (result.type === 'failure') Toast.error(String(result.data?.error ?? '요청을 처리하지 못했어요'));
          else if (result.type === 'error') Toast.error('요청을 처리하지 못했어요');
          else if (kind === 'reply') replyBody = '';
          await update();
        } finally {
          busy = null;
        }
      };
    };

  const handleReplyKeydown = (e: KeyboardEvent & { currentTarget: EventTarget & HTMLTextAreaElement }) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      e.currentTarget.form?.requestSubmit();
    }
  };

  // 패딩은 전 상태 동일 — 열림·닫힘 전환에서 안쪽 내용이 밀리면 안 된다. 강조는 보더 색·그림자가 담당한다.
  // 활성 전환 duration·이징은 화면 전체 공통(0.25s · 카드 이동과 같은 곡선) — 요소마다 다르면 어긋나 보인다.
  const cardRecipe = cva({
    base: {
      borderWidth: '1px',
      borderRadius: '10px',
      paddingX: '12px',
      paddingY: '10px',
      transition:
        '[border-color 0.25s cubic-bezier(0.2, 0, 0, 1), box-shadow 0.25s cubic-bezier(0.2, 0, 0, 1), background-color 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        active: { borderColor: 'border.brand', backgroundColor: 'surface.default', boxShadow: 'medium' },
        open: { borderColor: 'border.default', backgroundColor: 'surface.default', boxShadow: 'small' },
        settled: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
        settledActive: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
      },
      clickable: { true: { cursor: 'pointer' }, false: {} },
    },
  });

  // 원고 거터의 레일 번호 칩과 같은 시각 언어 — 양쪽의 같은 번호가 레일↔카드 연결 어포던스다.
  // 활성 반전도 레일 칩과 짝이다: 카드가 열리면 양쪽 번호가 같은 모습으로 반전된다.
  const numberChipRecipe = cva({
    base: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      flex: 'none',
      size: '16px',
      borderRadius: '5px',
      fontSize: '10px',
      fontWeight: 'bold',
      transition: '[background-color 0.25s cubic-bezier(0.2, 0, 0, 1), color 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        settled: { backgroundColor: 'surface.muted', color: 'text.faint' },
      },
      active: { true: {}, false: {} },
    },
    // 활성 반전도 계열을 따른다 — 레일 칩(ManuscriptView railChipRecipe)과 같은 규칙이다.
    compoundVariants: [
      { tone: 'open', active: true, css: { backgroundColor: 'accent.brand.default', color: 'text.bright' } },
      { tone: 'settled', active: true, css: { backgroundColor: 'border.strong', color: 'text.bright' } },
    ],
  });

  // 헤더 칩의 공통 치수 — 상태 칩과 회차 뱃지가 같은 형태로 나란히 선다.
  const chipBase = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    flex: 'none',
    paddingX: '8px',
    paddingY: '2px',
    borderWidth: '1px',
    borderRadius: 'full',
    fontSize: '11px',
    fontWeight: 'semibold',
  });

  // 신규 뱃지 — 마지막 리뷰에서 새로 생긴 피드백의 표지다. 몇 회차 출신인지는 표기하지 않는다(중요 정보가
  // 아니다 — 오너 확정). 상태 칩과 무게가 갈리도록 테두리 없는 소형 태그로, 축 라벨 오른쪽에 붙는다.
  const newBadgeClass = css({
    flex: 'none',
    paddingX: '5px',
    paddingY: '1px',
    borderRadius: '4px',
    backgroundColor: 'accent.brand.subtle',
    fontSize: '10px',
    fontWeight: 'semibold',
    color: 'text.brand',
  });

  // 상태 칩 — 세 상태가 같은 치수·형태·회색조를 쓴다. 해소도 튀지 않는다: 종결 상태끼리 무게가 갈릴 이유가
  // 없다(오너 확정 — 강조는 신규 뱃지의 몫이다).
  const stateChipRecipe = cva({
    base: chipBase,
    variants: {
      state: {
        closed: { borderColor: 'border.default', backgroundColor: 'surface.muted', color: 'text.subtle' },
        resolved: { borderColor: 'border.default', backgroundColor: 'surface.muted', color: 'text.subtle' },
        withdrawn: { borderColor: 'border.default', backgroundColor: 'surface.muted', color: 'text.subtle' },
      },
    },
  });

  const stateDotRecipe = cva({
    base: { size: '5px', flex: 'none', borderRadius: 'full' },
    variants: {
      state: {
        closed: { backgroundColor: 'text.faint' },
        resolved: { backgroundColor: 'text.faint' },
        withdrawn: { backgroundColor: 'text.faint' },
      },
    },
  });

  const quietLinkClass = css({
    flex: 'none',
    fontSize: '11px',
    fontWeight: 'semibold',
    color: 'text.brand',
    cursor: 'pointer',
    _hover: { color: 'accent.brand.hover' },
    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
  });

  const deleteLinkClass = css({
    flex: 'none',
    fontSize: '11px',
    color: 'text.faint',
    cursor: 'pointer',
    _hover: { color: 'text.danger' },
    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
  });

  const tone = $derived(settled ? (expanded ? 'settledActive' : 'settled') : expanded ? 'active' : 'open');

  // 펼침·접힘 콘텐츠는 grid-rows(0fr↔1fr)로 높이를 애니메이션한다 — 높이가 즉시 바뀌면 이웃 카드가
  // 밀려나기 전에 먼저 겹친다(카드 이동과 같은 0.25s·이징이라 성장과 밀림이 맞물린다). visibility는
  // 접힌 콘텐츠의 포커스·클릭을 막는 용도로, 전환 목록에 실어 열리는 동안에는 보이게 둔다.
  const revealRecipe = cva({
    base: {
      display: 'grid',
      transition: '[grid-template-rows 0.25s cubic-bezier(0.2, 0, 0, 1), visibility 0.25s]',
    },
    variants: {
      shown: {
        // visibility를 명시하지 않는다 — visible을 선언하면 상속을 끊어, 배치 전 카드 래퍼의
        // hidden(ThreadColumn positioned:false)을 뚫고 내용이 그려진다(실측: JS 로드 전 3줄 텍스트 띠).
        // 미선언이면 조상을 상속해 래퍼가 숨는 동안 함께 숨고, 평시엔 기본값 visible로 계산된다.
        true: { gridTemplateRows: '[1fr]' },
        false: { gridTemplateRows: '[0fr]', visibility: 'hidden' },
      },
    },
  });

  const revealInnerClass = css({ overflow: 'hidden', minHeight: '0' });

  // 카드 반응 칩 — 리뷰 반응(ReviewReaction thumbRecipe)과 같은 시각 언어다. 취사선택 표시라
  // 선택된 버튼의 재클릭은 해제로 간다(formaction — 서버는 설정·해제를 추론 없이 나눠 받는다).
  const reactionThumbRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '26px',
      borderWidth: '1px',
      borderRadius: '6px',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, border-color 0.15s ease]',
      _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
    },
    variants: {
      selected: {
        true: { borderColor: 'border.brand', backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        false: { borderColor: 'border.default', backgroundColor: 'surface.default', color: 'text.faint', _hover: { color: 'text.subtle' } },
      },
    },
  });

  // 헤더의 반응 표식 — 조작면이 아니라 남긴 반응의 현황이다. 번호 칩과 같은 16px 칩 언어를 쓴다.
  const reactionMarkRecipe = cva({
    base: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      flex: 'none',
      size: '16px',
      borderRadius: '5px',
    },
    variants: {
      value: {
        up: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        down: { backgroundColor: 'surface.muted', color: 'text.faint' },
      },
    },
  });

  // 총평이 이 지적을 어디에 놓았는지의 콜아웃 — 순서(급함)와 습관(같은 결)이 같은 급으로 나란히 선다.
  const calloutClass = css({
    marginTop: '10px',
    paddingX: '10px',
    paddingY: '7px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '6px',
    backgroundColor: 'surface.subtle',
    fontSize: '11px',
    lineHeight: '[1.55]',
    color: 'text.faint',
  });

  // 접힌 카드는 어디를 눌러도 열린다 — 헤더 버튼이 접근성 조작면이고, 이 핸들러는 포인터 편의다.
  // 내부 조작 요소에서 시작한 클릭은 제외하고, 펼친 카드는 본문 드래그·선택을 방해하지 않게 닫지 않는다.
  const openFromCard = (event: MouseEvent) => {
    if (expanded) return;
    if (event.target instanceof Element && event.target.closest('button, a, input, form')) return;
    onToggle();
  };

  // 삭제 확인 — 리뷰 중단(+page.svelte confirmCancel)과 같은 형태: 버튼은 다이얼로그만 열고 제출은 폼이 한다.
  let deleteForms = $state<Record<string, HTMLFormElement | undefined>>({});

  const confirmDelete = (commentId: string) => {
    Dialog.confirm({
      title: '답글을 지울까요?',
      message: '지운 답글은 되돌릴 수 없어요.',
      action: 'danger',
      actionLabel: '지우기',
      actionHandler: () => deleteForms[commentId]?.requestSubmit(),
    });
  };
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class={css(cardRecipe.raw({ tone, clickable: !expanded }))} data-thread-card={thread.id} onclick={openFromCard}>
  <button class={flex({ align: 'center', gap: '6px', width: 'full', cursor: 'pointer' })} onclick={onToggle} type="button">
    <span class={css(numberChipRecipe.raw({ tone: settled ? 'settled' : 'open', active: expanded }))}>{thread.issueIndex + 1}</span>
    <span
      class={css({
        minWidth: '0',
        fontSize: '13px',
        fontWeight: expanded ? 'bold' : 'semibold',
        color: settled ? 'text.faint' : 'text.default',
        textAlign: 'left',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
      })}
    >
      {thread.trait}
    </span>
    {#if isNew}
      <span class={newBadgeClass}>신규</span>
    {/if}
    <span class={flex({ align: 'center', gap: '6px', flex: 'none', marginLeft: 'auto' })}>
      {#if meta}
        <span class={css({ fontSize: '11px', color: 'text.faint', whiteSpace: 'nowrap' })}>{meta}</span>
      {/if}
      {#if thread.reaction}
        <span class={css(reactionMarkRecipe.raw({ value: thread.reaction }))}>
          <Icon icon={thread.reaction === 'up' ? IconThumbsUp : IconThumbsDown} size={10} />
        </span>
      {/if}
      {#if thread.state !== 'open'}
        <span class={css(stateChipRecipe.raw({ state: thread.state }))}>
          <span class={css(stateDotRecipe.raw({ state: thread.state }))}></span>
          {STATE_LABELS[thread.state]}
        </span>
      {/if}
    </span>
  </button>

  {#if !settled}
    <div class={css(revealRecipe.raw({ shown: !expanded }))}>
      <div class={revealInnerClass} data-reveal="snippet">
        <p
          class={css({
            marginTop: '5px',
            fontSize: '12px',
            lineHeight: '[1.5]',
            color: 'text.faint',
            lineClamp: '3',
          })}
        >
          {snippet}
        </p>
      </div>
    </div>
  {/if}

  <div class={css(revealRecipe.raw({ shown: expanded }))}>
    <div class={revealInnerClass} data-reveal="detail">
      {#if quote}
        <p
          class={css({
            marginTop: '10px',
            paddingX: '10px',
            paddingY: '7px',
            borderLeftWidth: '3px',
            borderColor: 'border.default',
            borderTopRightRadius: '4px',
            borderBottomRightRadius: '4px',
            backgroundColor: 'surface.subtle',
            fontFamily: 'RIDIBatang',
            fontSize: '13px',
            lineHeight: '[1.7]',
            color: 'text.subtle',
          })}
        >
          {quote}
        </p>
      {/if}

      {#if thread.body}
        <Paragraphs class={css({ marginTop: '9px', fontSize: '13px', lineHeight: '[1.65]', color: 'text.subtle' })} text={thread.body} />
      {/if}

      {#if pattern}
        <p class={calloutClass}>
          <span class={css({ fontWeight: 'bold', color: 'text.subtle' })}>반복되는 습관</span>
          · {pattern.theme} — 원고의 {pattern.count}곳에서 같은 습관이 보여요
        </p>
      {/if}

      {#if priority}
        <p class={calloutClass}>
          <span class={css({ fontWeight: 'bold', color: 'text.subtle' })}>손보실 순서</span>
          · {priority.total}가지 중 {priority.rank}번째 — {priority.body}
        </p>
      {/if}

      {#each shownComments as comment (comment.id)}
        <div class={flex({ gap: '8px', marginTop: '11px', paddingTop: '11px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
          <span
            class={flex({
              align: 'center',
              justify: 'center',
              flex: 'none',
              size: '22px',
              borderRadius: 'full',
              backgroundColor: 'surface.muted',
              fontSize: '10px',
              fontWeight: 'bold',
              color: 'text.faint',
            })}
          >
            {authorLabel(comment)}
          </span>
          <div class={css({ flexGrow: '1', minWidth: '0' })}>
            <div class={flex({ align: 'center', gap: '4px', fontSize: '11px', color: 'text.faint' })}>
              <span class={css({ fontWeight: 'semibold', color: 'text.subtle' })}>{authorLabel(comment)}</span>
              <span>·</span>
              <TimeAgo timestamp={comment.createdAt} />
              {#if comment.author === 'tester'}
                <form
                  bind:this={deleteForms[comment.id]}
                  class={css({ marginLeft: 'auto' })}
                  action="?/deleteReply"
                  method="post"
                  use:enhance={submit('delete')}
                >
                  <input name="commentId" type="hidden" value={comment.id} />
                  <button
                    class={deleteLinkClass}
                    disabled={busy !== null || locked}
                    onclick={() => confirmDelete(comment.id)}
                    type="button"
                  >
                    지우기
                  </button>
                </form>
              {/if}
            </div>
            <p class={css({ marginTop: '2px', fontSize: '12px', lineHeight: '[1.55]', color: 'text.subtle', whiteSpace: 'pre-wrap' })}>
              {comment.body}
            </p>
          </div>
        </div>
      {/each}

      <!-- 반응은 스레드 상태와 독립이다 — 닫힌 스레드에서도 남기고 바꿀 수 있다. -->
      <form
        class={flex({
          align: 'center',
          gap: '6px',
          marginTop: '11px',
          paddingTop: '11px',
          borderTopWidth: '1px',
          borderColor: 'border.subtle',
        })}
        action="?/reactThread"
        method="post"
        use:enhance={submit('react')}
      >
        <input name="threadId" type="hidden" value={thread.id} />
        <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '11px', color: 'text.faint' })}>이 피드백 어땠나요?</span>
        <button
          name="value"
          class={css(reactionThumbRecipe.raw({ selected: thread.reaction === 'up' }))}
          aria-label="좋았어요"
          aria-pressed={thread.reaction === 'up'}
          disabled={busy !== null}
          formaction={thread.reaction === 'up' ? '?/unreactThread' : undefined}
          type="submit"
          value="up"
        >
          <Icon icon={IconThumbsUp} size={12} />
        </button>
        <button
          name="value"
          class={css(reactionThumbRecipe.raw({ selected: thread.reaction === 'down' }))}
          aria-label="아쉬웠어요"
          aria-pressed={thread.reaction === 'down'}
          disabled={busy !== null}
          formaction={thread.reaction === 'down' ? '?/unreactThread' : undefined}
          type="submit"
          value="down"
        >
          <Icon icon={IconThumbsDown} size={12} />
        </button>
      </form>

      {#if !settled}
        {#if locked}
          <p class={css({ marginTop: '10px', fontSize: '12px', lineHeight: '[1.6]', color: 'text.faint' })}>
            리뷰가 진행되는 동안에는 답글을 남길 수 없어요
          </p>
        {:else}
          <form action="?/reply" method="post" use:enhance={submit('reply')}>
            <input name="threadId" type="hidden" value={thread.id} />
            <div
              class={flex({
                align: 'flex-end',
                gap: '6px',
                marginTop: '10px',
                paddingLeft: '10px',
                paddingRight: '4px',
                paddingY: '4px',
                borderWidth: '1px',
                borderColor: 'border.default',
                borderRadius: '6px',
                backgroundColor: 'surface.subtle',
              })}
            >
              <textarea
                name="body"
                class={css({
                  flexGrow: '1',
                  minWidth: '0',
                  paddingY: '4px',
                  maxHeight: '120px',
                  fontSize: '12px',
                  lineHeight: '[1.5]',
                  backgroundColor: 'transparent',
                  resize: 'none',
                  _placeholder: { color: 'text.faint' },
                  _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
                })}
                disabled={busy !== null}
                onkeydown={handleReplyKeydown}
                placeholder="답글은 다음 리뷰에 반영돼요"
                rows={1}
                bind:value={replyBody}
                use:autosize={{ value: replyBody }}></textarea>
              <button
                class={flex({
                  align: 'center',
                  justify: 'center',
                  flex: 'none',
                  size: '24px',
                  borderRadius: 'full',
                  backgroundColor: 'accent.brand.default',
                  color: 'text.bright',
                  cursor: 'pointer',
                  _disabled: { backgroundColor: 'interactive.disabled', color: 'text.disabled', cursor: 'not-allowed' },
                })}
                aria-label="답글 남기기"
                disabled={busy !== null}
                type="submit"
              >
                <Icon icon={IconArrowUp} size={12} />
              </button>
            </div>
          </form>
        {/if}

        <form
          class={flex({
            align: 'center',
            gap: '8px',
            marginTop: '12px',
            paddingTop: '12px',
            borderTopWidth: '1px',
            borderColor: 'border.subtle',
          })}
          action="?/close"
          method="post"
          use:enhance={submit('close')}
        >
          <input name="threadId" type="hidden" value={thread.id} />
          <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '11px', color: 'text.faint', textAlign: 'right' })}>
            다음 리뷰부터 다시 짚지 않아요 · 되돌릴 수 있어요
          </span>
          <button
            class={flex({
              align: 'center',
              gap: '6px',
              flex: 'none',
              height: '28px',
              paddingX: '11px',
              borderWidth: '1px',
              borderColor: 'border.default',
              borderRadius: '6px',
              backgroundColor: 'surface.default',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'text.subtle',
              boxShadow: 'small',
              cursor: 'pointer',
              _hover: { borderColor: 'border.strong' },
              _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
            })}
            disabled={busy !== null || locked}
            type="submit"
          >
            <span class={css({ display: 'inline-flex', color: 'text.success' })}>
              <Icon icon={IconCircleCheck} size={12} />
            </span>
            피드백 닫기
          </button>
        </form>
      {/if}
    </div>
  </div>

  {#if closed}
    <div
      class={flex({
        align: 'center',
        gap: '8px',
        marginTop: expanded ? '12px' : '7px',
        paddingTop: expanded ? '12px' : '0',
        borderTopWidth: expanded ? '1px' : '0',
        borderColor: 'border.subtle',
      })}
    >
      <span class={css({ fontSize: '11px', color: 'text.faint' })}>닫은 피드백은 다음 리뷰에서 다시 짚지 않아요</span>
      {#if reopenable}
        <form class={css({ marginLeft: 'auto' })} action="?/reopen" method="post" use:enhance={submit('reopen')}>
          <input name="threadId" type="hidden" value={thread.id} />
          <button class={quietLinkClass} disabled={busy !== null || locked} type="submit">다시 열기</button>
        </form>
      {/if}
    </div>
  {/if}
</div>
