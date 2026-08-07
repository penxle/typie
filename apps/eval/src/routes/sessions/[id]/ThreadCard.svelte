<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import IconArrowUp from '~icons/lucide/arrow-up';
  import IconCircleCheck from '~icons/lucide/circle-check';
  import { enhance } from '$app/forms';
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
    onToggle: () => void;
  };

  const { thread, comments, quote, pattern, priority, expanded, onToggle }: Props = $props();

  const closed = $derived(thread.state === 'closed');
  const snippet = $derived(thread.body ?? '');
  const meta = $derived(comments.length > 0 ? `댓글 ${comments.length}` : '');

  const authorLabel = (comment: Comment) => (comment.author === 'ai' ? 'AI' : '나');

  // 더블클릭이 댓글 2행·자기 유발 409를 만든다 — 왕복 중인 폼은 제출을 취소하고 조작면을 잠근다.
  let busy = $state<'reply' | 'close' | 'reopen' | 'delete' | null>(null);

  const submit =
    (kind: 'reply' | 'close' | 'reopen' | 'delete'): SubmitFunction =>
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
          await update();
        } finally {
          busy = null;
        }
      };
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
        closed: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
        closedActive: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
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
        closed: { backgroundColor: 'surface.muted', color: 'text.faint' },
      },
      active: { true: {}, false: {} },
    },
    // 활성 반전도 계열을 따른다 — 레일 칩(ManuscriptView railChipRecipe)과 같은 규칙이다.
    compoundVariants: [
      { tone: 'open', active: true, css: { backgroundColor: 'accent.brand.default', color: 'text.bright' } },
      { tone: 'closed', active: true, css: { backgroundColor: 'border.strong', color: 'text.bright' } },
    ],
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

  const tone = $derived(closed ? (expanded ? 'closedActive' : 'closed') : expanded ? 'active' : 'open');

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
    <span class={css(numberChipRecipe.raw({ tone: closed ? 'closed' : 'open', active: expanded }))}>{thread.issueIndex + 1}</span>
    <span
      class={css({
        minWidth: '0',
        fontSize: '13px',
        fontWeight: expanded ? 'bold' : 'semibold',
        color: closed ? 'text.faint' : 'text.default',
        textAlign: 'left',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
      })}
    >
      {thread.axis}
    </span>
    <span class={flex({ align: 'center', gap: '6px', flex: 'none', marginLeft: 'auto' })}>
      {#if meta}
        <span class={css({ fontSize: '11px', color: 'text.faint', whiteSpace: 'nowrap' })}>{meta}</span>
      {/if}
      {#if closed}
        <span
          class={flex({
            align: 'center',
            gap: '4px',
            flex: 'none',
            paddingX: '8px',
            paddingY: '2px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: 'full',
            backgroundColor: 'surface.muted',
            fontSize: '11px',
            fontWeight: 'semibold',
            color: 'text.subtle',
          })}
        >
          <span class={css({ size: '5px', flex: 'none', borderRadius: 'full', backgroundColor: 'text.faint' })}></span>
          닫힘
        </span>
      {/if}
    </span>
  </button>

  {#if !closed}
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
        <p class={css({ marginTop: '9px', fontSize: '13px', lineHeight: '[1.65]', color: 'text.subtle' })}>{thread.body}</p>
      {/if}

      {#if pattern}
        <p class={calloutClass}>
          <span class={css({ fontWeight: 'bold', color: 'text.subtle' })}>반복되는 습관</span>
          · {pattern.theme} — 원고 전체에서 {pattern.count}건이 같은 결이에요
        </p>
      {/if}

      {#if priority}
        <p class={calloutClass}>
          <span class={css({ fontWeight: 'bold', color: 'text.subtle' })}>손보실 순서</span>
          · 전체 {priority.total}단계 중 {priority.rank}번째 — {priority.body}
        </p>
      {/if}

      {#each comments as comment (comment.id)}
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
                  <button class={deleteLinkClass} disabled={busy !== null} onclick={() => confirmDelete(comment.id)} type="button">
                    지우기
                  </button>
                </form>
              {/if}
            </div>
            <p class={css({ marginTop: '2px', fontSize: '12px', lineHeight: '[1.55]', color: 'text.subtle' })}>{comment.body}</p>
          </div>
        </div>
      {/each}

      {#if !closed}
        <form action="?/reply" method="post" use:enhance={submit('reply')}>
          <input name="threadId" type="hidden" value={thread.id} />
          <div
            class={flex({
              align: 'center',
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
            <input
              name="body"
              class={css({
                flexGrow: '1',
                minWidth: '0',
                height: '26px',
                fontSize: '12px',
                backgroundColor: 'transparent',
                _placeholder: { color: 'text.faint' },
                _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
              })}
              disabled={busy !== null}
              placeholder="답글은 다음 리뷰에 반영돼요"
              type="text"
            />
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
            다음 회차부터 다시 짚지 않아요 · 되돌릴 수 있어요
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
            disabled={busy !== null}
            type="submit"
          >
            <span class={css({ display: 'inline-flex', color: 'text.success' })}>
              <Icon icon={IconCircleCheck} size={12} />
            </span>
            스레드 닫기
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
      <span class={css({ fontSize: '11px', color: 'text.faint' })}>닫힌 스레드는 다음 회차에서 다시 짚지 않아요</span>
      <form class={css({ marginLeft: 'auto' })} action="?/reopen" method="post" use:enhance={submit('reopen')}>
        <input name="threadId" type="hidden" value={thread.id} />
        <button class={quietLinkClass} disabled={busy !== null} type="submit">다시 열기</button>
      </form>
    </div>
  {/if}
</div>
