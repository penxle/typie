<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize, tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, TimeAgo } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import ArchiveIcon from '~icons/lucide/archive';
  import ArchiveRestoreIcon from '~icons/lucide/archive-restore';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PencilIcon from '~icons/lucide/pencil';
  import ThumbsDownIcon from '~icons/lucide/thumbs-down';
  import ThumbsUpIcon from '~icons/lucide/thumbs-up';
  import Trash2Icon from '~icons/lucide/trash-2';
  import XIcon from '~icons/lucide/x';
  import PrismIcon from '~icons/typie/prism';
  import { Img } from '$lib/components';
  import { getMarginContext } from './context.svelte.ts';
  import { cardHeaderSlot } from './margin-view.ts';
  import type { MarginComment, MarginItem } from './context.svelte.ts';

  // scrollable은 팝오버가 준 높이를 카드가 다 쓰고 넘치는 만큼을 제 안에서 접는다는 뜻이다.
  // 컬럼에서는 켜지 않는다 — 거기서는 카드가 제 높이를 다 차지하고 컬럼이 통째로 스크롤된다.
  // onClose의 X는 팝오버에서는 카드를 닫고 컬럼에서는 펼친 카드를 접는다 — 접힌 카드에는 접을 것이 없어 펼쳤을 때만 선다
  type Props = { item: MarginItem; expanded: boolean; onToggle: () => void; onClose?: () => void; scrollable?: boolean };
  let { item, expanded, onToggle, onClose, scrollable = false }: Props = $props();

  const margin = getMarginContext();
  const thread = $derived(item.thread);
  // 닫힘(작가가 접음)·해소·철회(재리뷰 처분)는 조작면과 흐림 처리를 공유한다 — 되돌리기는 닫힘만의 몫이다
  const settled = $derived(thread !== null && thread.state !== 'OPEN');
  const reaction = $derived(thread?.reaction ?? null);

  // 답글만이 행을 새로 만든다 — 재진입을 막을 곳도 여기뿐이다
  let sending = $state(false);
  let reply = $state('');
  // 수정은 그 답글 자리에서 한다 — 아래 입력창은 새 답글 전용이라 쓰던 글이 밀려나지 않는다
  let editingId = $state<string | null>(null);
  let editDraft = $state('');

  // 거부를 삼켜 "처리된 실패"로 표시한다 — 흘려보내면 Sentry가 잡히지 않은 거부로 다시 올린다
  const run = async (action: () => Promise<void>) => {
    try {
      await action();
    } catch {
      // 토스트는 컨트롤러가 띄운다
    }
  };

  const canSubmit = $derived(!sending && reply.trim().length > 0);

  const submitReply = async () => {
    if (!thread || !canSubmit) return;
    const text = reply.trim();
    sending = true;
    await run(async () => {
      await margin.reply(thread.id, text);
      reply = '';
    });
    sending = false;
  };

  const cancelEdit = () => {
    editingId = null;
    editDraft = '';
  };

  const submitEdit = async (commentId: string) => {
    const text = editDraft.trim();
    if (text.length === 0) return;
    await run(async () => {
      await margin.editComment(commentId, text);
      cancelEdit();
    });
  };

  const focusOnMount = (node: HTMLTextAreaElement) => {
    node.focus();
    node.setSelectionRange(node.value.length, node.value.length);
  };

  // 빈 본문 코멘트는 그리지 않는다 — 아바타와 시각만 남은 껍데기 줄이 대화에 끼어들면 안 된다
  const shownComments = $derived((thread?.comments ?? []).filter((comment) => comment.body.trim().length > 0));
  const slot = $derived(cardHeaderSlot(thread?.state ?? null, expanded, shownComments.length));
  // 대화 층은 담을 것이 있을 때만 선다 — 종결 + 댓글 0이면 헤어라인만 남은 빈 구획이 된다
  const talkShown = $derived(thread !== null && (shownComments.length > 0 || !settled));

  const quote = $derived(thread?.quote ?? item.strength?.quote ?? '');
  const body = $derived(thread?.body ?? item.strength?.body ?? '');
  // 줄바꿈(문단 경계)을 화면 간격으로 세운다 — pre-line은 줄만 바꿔 문단이 붙어 보인다(오너 관측 2026-08-17)
  const paragraphs = $derived(
    body
      .split('\n')
      .map((paragraph) => paragraph.trim())
      .filter((paragraph) => paragraph.length > 0),
  );

  const STATE_LABELS = { CLOSED: '닫음', RESOLVED: '해소됨', WITHDRAWN: '철회됨' };
  const stateLabel = $derived(thread === null || thread.state === 'OPEN' ? null : STATE_LABELS[thread.state]);

  // 사람 댓글은 실제 작성자를 세운다 — 문서를 함께 쓰는 사람이 남긴 답글과 내 답글이 갈려야 한다
  const authorLabel = (comment: MarginComment) => (comment.author === 'AI' ? 'AI' : comment.user.name);

  // 지운 답글은 서버에서 되돌릴 수 없다 — 한 번의 오클릭이 남의 글을 지우게 두지 않는다
  const confirmDelete = (commentId: string) => {
    Dialog.confirm({
      title: '답글을 지울까요?',
      message: '지운 답글은 되돌릴 수 없어요.',
      action: 'danger',
      actionLabel: '지우기',
      actionHandler: () => {
        void run(() => margin.deleteComment(commentId));
      },
    });
  };

  // 패딩은 전 상태 동일 — 열림·닫힘 전환에서 안쪽 내용이 밀리면 안 된다. 강조는 보더 색·그림자가 담당한다.
  // 활성 전환 duration·이징은 화면 전체 공통(0.25s · 카드 이동과 같은 곡선) — 요소마다 다르면 어긋나 보인다
  const cardRecipe = cva({
    base: {
      borderWidth: '1px',
      borderRadius: '10px',
      padding: '12px',
      // 에디터 표면은 user-select:none이다 — 카드가 어디에 마운트되든 제 글자는 고를 수 있어야 한다
      userSelect: 'text',
      WebkitUserSelect: 'text',
      transition:
        '[border-color 0.25s cubic-bezier(0.2, 0, 0, 1), box-shadow 0.25s cubic-bezier(0.2, 0, 0, 1), background-color 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        active: { borderColor: 'review.issue.default', backgroundColor: 'surface.default', boxShadow: 'medium' },
        activeStrength: { borderColor: 'review.strength.default', backgroundColor: 'surface.default', boxShadow: 'medium' },
        open: { borderColor: 'border.default', backgroundColor: 'surface.default', boxShadow: 'small' },
        settled: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
        settledActive: { borderColor: 'border.subtle', backgroundColor: 'surface.subtle' },
      },
      clickable: { true: { cursor: 'pointer' }, false: {} },
    },
  });

  // 자리를 잃은 카드도 종결과 같은 회색조로 내려간다 — 지금 원고에 그을 자리가 없다는 뜻이다
  const dimmed = $derived(settled || !item.anchored);
  const strength = $derived(item.kind === 'strength');
  const tone = $derived(dimmed ? (expanded ? 'settledActive' : 'settled') : expanded ? (strength ? 'activeStrength' : 'active') : 'open');

  // 원고 거터의 레일 번호 칩과 같은 시각 언어 — 양쪽의 같은 번호가 레일↔카드 연결 어포던스다.
  // 활성 반전도 레일 칩과 짝이다: 카드가 열리면 양쪽 번호가 같은 모습으로 반전된다
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
        open: { backgroundColor: 'review.issue.subtle', color: 'review.issue.default' },
        strength: {
          backgroundColor: 'review.strength.subtle',
          color: 'review.strength.default',
        },
        settled: { backgroundColor: 'surface.muted', color: 'text.faint' },
      },
      active: { true: {}, false: {} },
    },
    compoundVariants: [
      { tone: 'open', active: true, css: { backgroundColor: 'review.issue.default', color: 'surface.default' } },
      { tone: 'strength', active: true, css: { backgroundColor: 'review.strength.default', color: 'surface.default' } },
      { tone: 'settled', active: true, css: { backgroundColor: 'border.strong', color: 'text.bright' } },
    ],
  });

  // 접힘·펼침 콘텐츠는 상시 DOM에 둔다 — 컬럼이 최종 높이를 scrollHeight로 예측한다.
  // visibility는 접힌 콘텐츠의 포커스·클릭을 막는 용도로, 전환 목록에 실어 열리는 동안에는 보이게 둔다
  const revealRecipe = cva({
    base: { display: 'grid', transition: '[grid-template-rows 0.25s cubic-bezier(0.2, 0, 0, 1), visibility 0.25s]' },
    variants: {
      shown: {
        // visibility를 명시하지 않는다 — visible을 선언하면 상속을 끊어, 배치 전 카드 래퍼의
        // hidden(PrismCardColumn positioned:false)을 뚫고 내용이 그려진다
        true: { gridTemplateRows: '[1fr]' },
        false: { gridTemplateRows: '[0fr]', visibility: 'hidden' },
      },
    },
  });

  const revealInnerClass = css({ overflow: 'hidden', minHeight: '0' });

  const editAreaClass = css({
    width: 'full',
    marginTop: '4px',
    paddingX: '10px',
    paddingY: '6px',
    maxHeight: '120px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '6px',
    backgroundColor: 'surface.subtle',
    fontSize: '12px',
    lineHeight: '[1.55]',
    color: 'text.subtle',
    resize: 'none',
  });

  const editCancelClass = css({
    flex: 'none',
    fontSize: '11px',
    fontWeight: 'semibold',
    color: 'text.faint',
    cursor: 'pointer',
    _hover: { color: 'text.subtle' },
  });

  const editSaveClass = css({
    flex: 'none',
    fontSize: '11px',
    fontWeight: 'semibold',
    color: 'text.brand',
    cursor: 'pointer',
    _hover: { color: 'accent.brand.hover' },
    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
  });

  // 카드 반응 — 취사선택 표시라 선택된 버튼의 재클릭은 해제로 간다
  const reactionThumbRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      flex: 'none',
      size: '18px',
      borderRadius: '4px',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, color 0.15s ease]',
      _hover: { backgroundColor: 'surface.muted' },
      _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
    },
    variants: {
      selected: {
        true: { color: 'text.brand' },
        false: { color: 'text.faint', _hover: { color: 'text.subtle' } },
      },
    },
  });

  // 총평이 이 지적을 어디에 놓았는지의 콜아웃 — 순서(급함)와 습관(같은 결)이 같은 급으로 나란히 선다
  const calloutClass = css({
    marginTop: '12px',
    paddingX: '12px',
    paddingY: '8px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '6px',
    backgroundColor: 'surface.subtle',
    fontSize: '11px',
    lineHeight: '[1.55]',
    color: 'text.faint',
  });

  const calloutLeadClass = css({ fontWeight: 'bold', color: 'text.subtle' });

  // 아이콘만 남긴 헤더 버튼 — 음수 마진으로 24px 히트 영역을 쓰면서 헤더 높이는 칩 높이에 맞춘다.
  // 카드 패딩을 파고드는(flush) 것은 오른쪽 끝 버튼만이다 — 중간 버튼까지 파고들면 이웃 버튼과 붙는다
  const headerActionRecipe = cva({
    base: {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      flex: 'none',
      size: '24px',
      marginY: '-4px',
      borderRadius: '6px',
      color: 'text.faint',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, color 0.15s ease]',
      _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
      _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
    },
    variants: {
      flush: { true: { marginRight: '-4px' }, false: {} },
    },
  });

  // 접힌 카드는 어디를 눌러도 열린다 — 헤더 버튼이 접근성 조작면이고, 이 핸들러는 포인터 편의다.
  // 내부 조작 요소에서 시작한 클릭은 제외하고, 펼친 카드는 본문 드래그·선택을 방해하지 않게 닫지 않는다
  const openFromCard = (event: MouseEvent) => {
    if (expanded) return;
    if (event.target instanceof Element && event.target.closest('button, a, input, form')) return;
    onToggle();
  };
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class={css(
    cardRecipe.raw({ tone, clickable: !expanded }),
    scrollable ? { minHeight: '0', overflowY: 'auto', overscrollBehavior: 'contain' } : {},
  )}
  data-prism-card={item.id}
  onclick={openFromCard}
>
  <div class={flex({ align: 'center', gap: '8px' })}>
    <button
      class={flex({ align: 'center', gap: '8px', flexGrow: '1', minWidth: '0', cursor: 'pointer' })}
      aria-expanded={expanded}
      onclick={onToggle}
      type="button"
    >
      <span class={css(numberChipRecipe.raw({ tone: dimmed ? 'settled' : strength ? 'strength' : 'open', active: expanded }))}>
        {item.number}
      </span>
      <span
        class={css({
          minWidth: '0',
          fontSize: '13px',
          fontWeight: expanded ? 'bold' : 'semibold',
          color: dimmed ? 'text.faint' : 'text.default',
          textAlign: 'left',
          truncate: true,
        })}
      >
        {thread?.trait ?? '잘 닿은 대목'}
      </span>

      {#if slot.comments || slot.state}
        <span class={flex({ align: 'center', gap: '8px', flex: 'none', marginLeft: 'auto', fontSize: '11px', color: 'text.faint' })}>
          {#if slot.comments}
            <span class={css({ whiteSpace: 'nowrap' })}>댓글 {shownComments.length}</span>
          {/if}
          {#if slot.state && stateLabel}
            <span class={css({ fontWeight: 'semibold', whiteSpace: 'nowrap' })}>{stateLabel}</span>
          {/if}
        </span>
      {/if}
    </button>

    {#if slot.action && thread}
      <button
        class={css(headerActionRecipe.raw({ flush: !onClose }))}
        aria-label={slot.action === 'close' ? '다음 회차부터 다시 짚지 않기' : '다음 회차부터 다시 짚기'}
        onclick={() => void run(() => (slot.action === 'close' ? margin.close(thread.id) : margin.reopen(thread.id)))}
        type="button"
        use:tooltip={{
          message: slot.action === 'close' ? '다음 회차부터 다시 짚지 않기' : '다음 회차부터 다시 짚기',
          delay: 200,
          arrow: false,
        }}
      >
        <Icon icon={slot.action === 'close' ? ArchiveIcon : ArchiveRestoreIcon} size={16} />
      </button>
    {/if}

    {#if onClose && expanded}
      <button
        class={css(headerActionRecipe.raw({ flush: true }))}
        aria-label="닫기"
        onclick={onClose}
        type="button"
        use:tooltip={{
          message: '닫기',
          delay: 200,
          arrow: false,
        }}
      >
        <Icon icon={XIcon} size={16} />
      </button>
    {/if}
  </div>

  {#if !settled}
    <div class={css(revealRecipe.raw({ shown: !expanded }))}>
      <div class={revealInnerClass} data-reveal="snippet">
        <p class={css({ marginTop: '8px', fontSize: '12px', lineHeight: '[1.5]', color: 'text.faint', lineClamp: '3' })}>
          {body}
        </p>
      </div>
    </div>
  {/if}

  <div class={css(revealRecipe.raw({ shown: expanded }))}>
    <div class={revealInnerClass} data-reveal="detail">
      {#if quote}
        <p
          class={css({
            marginTop: '12px',
            paddingX: '12px',
            paddingY: '8px',
            borderLeftWidth: '3px',
            borderColor: 'border.default',
            borderTopRightRadius: '4px',
            borderBottomRightRadius: '4px',
            backgroundColor: 'surface.subtle',
            fontFamily: 'prose',
            fontSize: '13px',
            lineHeight: '[1.7]',
            color: 'text.subtle',
          })}
        >
          {quote}
        </p>
      {/if}

      {#if body}
        <div
          class={css({
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
            marginTop: '12px',
            fontSize: '13px',
            lineHeight: '[1.65]',
            color: 'text.subtle',
          })}
        >
          {#each paragraphs as paragraph, index (index)}
            <p>{paragraph}</p>
          {/each}
        </div>
      {/if}

      {#if item.callouts.pattern}
        <p class={calloutClass}>
          <span class={calloutLeadClass}>반복되는 습관</span>
          · {item.callouts.pattern.theme}. 원고의 {item.callouts.pattern.count}곳에서 같은 습관이 보여요
        </p>
      {/if}

      {#if item.callouts.priority}
        <p class={calloutClass}>
          <span class={calloutLeadClass}>손보실 순서</span>
          · {item.callouts.priority.total}가지 중 {item.callouts.priority.rank}번째. {item.callouts.priority.body}
        </p>
      {/if}

      {#if talkShown}
        <div class={css({ marginTop: '20px', paddingTop: '16px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
          {#each shownComments as comment (comment.id)}
            <div class={flex({ gap: '8px', marginTop: '12px', _first: { marginTop: '0' } })}>
              {#if comment.author === 'AI'}
                <span
                  class={flex({
                    align: 'center',
                    justify: 'center',
                    flex: 'none',
                    size: '22px',
                    borderRadius: 'full',
                    backgroundColor: 'surface.muted',
                    color: 'text.faint',
                  })}
                >
                  <Icon icon={PrismIcon} size={12} />
                </span>
              {:else}
                <Img
                  style={css.raw({ flex: 'none', size: '22px', borderRadius: 'full' })}
                  alt={comment.user.name}
                  image$key={comment.user.avatar}
                  size={24}
                />
              {/if}
              <div class={css({ flexGrow: '1', minWidth: '0' })}>
                <div class={flex({ align: 'center', gap: '4px', fontSize: '11px', color: 'text.faint' })}>
                  <span class={css({ minWidth: '0', fontWeight: 'semibold', color: 'text.subtle', truncate: true })}>
                    {authorLabel(comment)}
                  </span>
                  <span>·</span>
                  <TimeAgo style={css.raw({ flex: 'none' })} timestamp={new Date(comment.createdAt).getTime()} />
                  {#if comment.author === 'USER' && comment.user.id === margin.myId}
                    <Menu
                      style={flex.raw({
                        align: 'center',
                        justify: 'center',
                        flex: 'none',
                        marginLeft: 'auto',
                        size: '18px',
                        borderRadius: '4px',
                        color: 'text.faint',
                        cursor: 'pointer',
                        _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
                      })}
                      placement="bottom-end"
                    >
                      {#snippet button()}
                        <Icon icon={EllipsisIcon} size={14} />
                        <span class={css({ srOnly: true })}>답글 메뉴</span>
                      {/snippet}
                      {#snippet children({ close })}
                        <MenuItem
                          icon={PencilIcon}
                          onclick={() => {
                            editingId = comment.id;
                            editDraft = comment.body;
                            close();
                          }}
                        >
                          수정
                        </MenuItem>
                        <MenuItem
                          icon={Trash2Icon}
                          onclick={() => {
                            close();
                            confirmDelete(comment.id);
                          }}
                          variant="danger"
                        >
                          지우기
                        </MenuItem>
                      {/snippet}
                    </Menu>
                  {/if}
                </div>
                {#if editingId === comment.id}
                  <textarea
                    class={editAreaClass}
                    onkeydown={(event) => {
                      if (event.key === 'Escape') {
                        // 팝오버의 Escape로 새어 나가면 카드가 닫히며 고치던 글이 사라진다
                        event.stopPropagation();
                        if (event.isComposing) return;
                        cancelEdit();
                        return;
                      }
                      if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
                        event.preventDefault();
                        void submitEdit(comment.id);
                      }
                    }}
                    rows={1}
                    bind:value={editDraft}
                    use:autosize={{ value: editDraft }}
                    use:focusOnMount></textarea>
                  <div class={flex({ align: 'center', justify: 'flex-end', gap: '10px', marginTop: '6px' })}>
                    <button class={editCancelClass} onclick={cancelEdit} type="button">취소</button>
                    <button
                      class={editSaveClass}
                      disabled={editDraft.trim().length === 0}
                      onclick={() => void submitEdit(comment.id)}
                      type="button"
                    >
                      저장
                    </button>
                  </div>
                {:else}
                  <p
                    class={css({ marginTop: '2px', fontSize: '12px', lineHeight: '[1.55]', color: 'text.subtle', whiteSpace: 'pre-wrap' })}
                  >
                    {comment.body}
                  </p>
                {/if}
              </div>
            </div>
          {/each}

          {#if !settled}
            <div
              class={flex({
                align: 'flex-end',
                gap: '6px',
                marginTop: '12px',
                // 댓글이 없으면 입력이 대화 층의 첫 요소다 — 래퍼 패딩에 제 여백을 더하면 홀로 28px가 된다
                _first: { marginTop: '0' },
                paddingLeft: '12px',
                paddingRight: '4px',
                paddingY: '4px',
                borderWidth: '1px',
                borderColor: 'border.default',
                borderRadius: '6px',
                backgroundColor: 'surface.subtle',
              })}
            >
              <textarea
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
                disabled={sending}
                onkeydown={(event) => {
                  if (event.key === 'Escape') {
                    // 팝오버의 Escape로 새어 나가면 카드가 닫히며 쓰던 답글이 사라진다
                    event.stopPropagation();
                    return;
                  }
                  if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
                    event.preventDefault();
                    void submitReply();
                  }
                }}
                placeholder="답글은 다음 리뷰에 반영돼요"
                rows={1}
                bind:value={reply}
                use:autosize={{ value: reply }}></textarea>
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
                disabled={!canSubmit}
                onclick={() => void submitReply()}
                type="button"
              >
                <Icon icon={ArrowUpIcon} size={12} />
              </button>
            </div>
          {/if}
        </div>
      {/if}

      {#if thread}
        <!-- 반응은 스레드 상태와 독립이다 — 닫힌 스레드에서도 남기고 바꿀 수 있다 -->
        <div
          class={css(
            flex.raw({ align: 'center', gap: '6px' }),
            talkShown
              ? { marginTop: '16px' }
              : { marginTop: '20px', paddingTop: '16px', borderTopWidth: '1px', borderColor: 'border.subtle' },
          )}
        >
          <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '11px', color: 'text.faint' })}>이 피드백 어땠나요?</span>
          <button
            class={css(reactionThumbRecipe.raw({ selected: reaction === 'UP' }))}
            aria-label="좋았어요"
            aria-pressed={reaction === 'UP'}
            onclick={() => void run(() => margin.react(thread.id, reaction === 'UP' ? null : 'UP'))}
            type="button"
          >
            <Icon icon={ThumbsUpIcon} size={14} />
          </button>
          <button
            class={css(reactionThumbRecipe.raw({ selected: reaction === 'DOWN' }))}
            aria-label="아쉬웠어요"
            aria-pressed={reaction === 'DOWN'}
            onclick={() => void run(() => margin.react(thread.id, reaction === 'DOWN' ? null : 'DOWN'))}
            type="button"
          >
            <Icon icon={ThumbsDownIcon} size={14} />
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>
