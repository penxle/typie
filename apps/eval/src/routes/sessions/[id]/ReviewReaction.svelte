<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import IconArrowUp from '~icons/lucide/arrow-up';
  import IconThumbsDown from '~icons/lucide/thumbs-down';
  import IconThumbsUp from '~icons/lucide/thumbs-up';
  import { enhance } from '$app/forms';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { PageData } from './$types';

  // 리뷰 반응은 두 자리에 선다 — 패널 푸터(compact)와 드로어 편지 말미(letter). 같은 액션(?/react)을
  // 같은 문법으로 쏘고, 조판만 자리에 맞춘다. 반응을 남기면 노트 입력이 아래로 펼쳐진다(기존 밴드 동작).
  type Props = {
    reaction: PageData['reaction'];
    variant: 'panel' | 'letter';
  };

  const { reaction, variant }: Props = $props();

  // 저장된 노트는 일반 텍스트로 굳고, 수정 버튼이 다시 입력으로 되돌린다 — 제출 성공이 유일한 굳힘 지점이다.
  let editing = $state(false);

  const submitReact: SubmitFunction = () => {
    return async ({ result, update }) => {
      if (result.type === 'failure') Toast.error(String(result.data?.error ?? '반응을 남기지 못했어요'));
      else if (result.type === 'error') Toast.error('반응을 남기지 못했어요');
      await update({ reset: false });
    };
  };

  const submitNote: SubmitFunction = () => {
    return async ({ result, update }) => {
      if (result.type === 'failure') Toast.error(String(result.data?.error ?? '반응을 남기지 못했어요'));
      else if (result.type === 'error') Toast.error('반응을 남기지 못했어요');
      else editing = false;
      await update({ reset: false });
    };
  };

  const handleNoteKeydown = (e: KeyboardEvent & { currentTarget: EventTarget & HTMLTextAreaElement }) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      e.currentTarget.form?.requestSubmit();
    }
  };

  const thumbRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderWidth: '1px',
      borderRadius: '6px',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, border-color 0.15s ease]',
    },
    variants: {
      size: {
        panel: { size: '26px' },
        letter: { size: '30px', borderRadius: '7px' },
      },
      selected: {
        true: { borderColor: 'border.brand', backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        false: { borderColor: 'border.default', backgroundColor: 'surface.default', color: 'text.faint', _hover: { color: 'text.subtle' } },
      },
    },
  });

  const labelRecipe = cva({
    base: { flexGrow: '1', minWidth: '0' },
    variants: {
      size: {
        panel: { fontSize: '11px', color: 'text.faint' },
        letter: { fontSize: '12px', fontWeight: 'semibold', color: 'text.default' },
      },
    },
  });

  const wrapRecipe = cva({
    variants: {
      size: {
        panel: {},
        letter: {
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '10px',
          backgroundColor: 'surface.default',
          paddingX: '18px',
          paddingY: '16px',
        },
      },
    },
  });
</script>

<div class={css(wrapRecipe.raw({ size: variant }))}>
  <form class={flex({ align: 'center', gap: '6px' })} action="?/react" method="post" use:enhance={submitReact}>
    <input name="note" type="hidden" value={reaction?.note ?? ''} />
    <span class={css(labelRecipe.raw({ size: variant }))}>이번 리뷰 어땠나요?</span>
    <!-- 선택된 버튼의 재클릭은 해제다 — 서버가 같은 값으로 추론하지 않고(메모 폼이 같은 값을 재전송한다)
         클라이언트가 formaction으로 해제 액션을 지목한다. -->
    <button
      name="value"
      class={css(thumbRecipe.raw({ size: variant, selected: reaction?.value === 'up' }))}
      aria-label="좋았어요"
      aria-pressed={reaction?.value === 'up'}
      formaction={reaction?.value === 'up' ? '?/unreact' : undefined}
      type="submit"
      value="up"
    >
      <Icon icon={IconThumbsUp} size={12} />
    </button>
    <button
      name="value"
      class={css(thumbRecipe.raw({ size: variant, selected: reaction?.value === 'down' }))}
      aria-label="아쉬웠어요"
      aria-pressed={reaction?.value === 'down'}
      formaction={reaction?.value === 'down' ? '?/unreact' : undefined}
      type="submit"
      value="down"
    >
      <Icon icon={IconThumbsDown} size={12} />
    </button>
  </form>

  {#if reaction && reaction.note && !editing}
    <div class={flex({ align: 'center', gap: '8px', marginTop: '10px' })}>
      <p
        class={css({ flexGrow: '1', minWidth: '0', fontSize: '12px', lineHeight: '[1.55]', color: 'text.subtle', whiteSpace: 'pre-wrap' })}
      >
        {reaction.note}
      </p>
      <button
        class={css({
          flex: 'none',
          fontSize: '11px',
          fontWeight: 'semibold',
          color: 'text.brand',
          cursor: 'pointer',
          _hover: { color: 'accent.brand.hover' },
        })}
        onclick={() => (editing = true)}
        type="button"
      >
        수정
      </button>
    </div>
  {:else if reaction}
    <form class={flex({ marginTop: '10px' })} action="?/react" method="post" use:enhance={submitNote}>
      <input name="value" type="hidden" value={reaction.value} />
      <div
        class={flex({
          align: 'flex-end',
          gap: '6px',
          width: 'full',
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
          name="note"
          class={css({
            flexGrow: '1',
            minWidth: '0',
            paddingY: '3px',
            maxHeight: '120px',
            fontSize: '12px',
            lineHeight: '[1.5]',
            backgroundColor: 'transparent',
            resize: 'none',
            _placeholder: { color: 'text.faint' },
          })}
          onkeydown={handleNoteKeydown}
          placeholder="몇 자 덧붙이기"
          rows={1}
          value={reaction.note ?? ''}
          use:autosize></textarea>
        <button
          class={flex({
            align: 'center',
            justify: 'center',
            flex: 'none',
            size: '22px',
            borderRadius: 'full',
            backgroundColor: 'accent.brand.default',
            color: 'text.bright',
            cursor: 'pointer',
          })}
          aria-label="남기기"
          type="submit"
        >
          <Icon icon={IconArrowUp} size={10} />
        </button>
      </div>
    </form>
  {/if}
</div>
