<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import IconArrowUp from '~icons/lucide/arrow-up';
  import IconThumbsDown from '~icons/lucide/thumbs-down';
  import IconThumbsUp from '~icons/lucide/thumbs-up';
  import { enhance } from '$app/forms';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { PageData } from './$types';

  // 리뷰 반응은 두 자리에 선다 — 패널 푸터(compact)와 드로어 편지 말미(letter). 같은 액션(?/react)을
  // 같은 문법으로 쏘고, 조판만 자리에 맞춘다. 반응을 남기면 한 줄 입력이 아래로 펼쳐진다(기존 밴드 동작).
  type Props = {
    reaction: PageData['reaction'];
    variant: 'panel' | 'letter';
  };

  const { reaction, variant }: Props = $props();

  const submitReact: SubmitFunction = () => {
    return async ({ result, update }) => {
      if (result.type === 'failure') Toast.error(String(result.data?.error ?? '반응을 남기지 못했어요'));
      else if (result.type === 'error') Toast.error('반응을 남기지 못했어요');
      await update({ reset: false });
    };
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
    <button
      name="value"
      class={css(thumbRecipe.raw({ size: variant, selected: reaction?.value === 'up' }))}
      aria-label="좋았어요"
      type="submit"
      value="up"
    >
      <Icon icon={IconThumbsUp} size={12} />
    </button>
    <button
      name="value"
      class={css(thumbRecipe.raw({ size: variant, selected: reaction?.value === 'down' }))}
      aria-label="아쉬웠어요"
      type="submit"
      value="down"
    >
      <Icon icon={IconThumbsDown} size={12} />
    </button>
  </form>

  {#if reaction}
    <form class={flex({ marginTop: '10px' })} action="?/react" method="post" use:enhance={submitReact}>
      <input name="value" type="hidden" value={reaction.value} />
      <div
        class={flex({
          align: 'center',
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
        <input
          name="note"
          class={css({
            flexGrow: '1',
            minWidth: '0',
            height: '24px',
            fontSize: '12px',
            backgroundColor: 'transparent',
            _placeholder: { color: 'text.faint' },
          })}
          placeholder="한 줄 덧붙이기"
          type="text"
          value={reaction.note ?? ''}
        />
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
          aria-label="한 줄 남기기"
          type="submit"
        >
          <Icon icon={IconArrowUp} size={10} />
        </button>
      </div>
    </form>
  {/if}
</div>
