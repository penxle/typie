<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';

  type Props = {
    open: boolean;
    onconfirm?: (mode: 'html' | 'text') => void;
  };

  let { open = $bindable(), onconfirm }: Props = $props();
</script>

<Modal style={css.raw({ maxWidth: '400px' })} bind:open>
  <div class={css({ padding: '24px' })}>
    <h2
      class={css({
        fontSize: '18px',
        fontWeight: 'semibold',
        marginBottom: '12px',
        color: 'text.default',
      })}
    >
      붙여넣기 옵션
    </h2>

    <p
      class={css({
        fontSize: '14px',
        color: 'text.subtle',
        marginBottom: '20px',
        lineHeight: '[1.5]',
      })}
    >
      텍스트를 어떤 형식으로 붙여넣으시겠어요?
    </p>

    <div
      class={flex({
        direction: 'column',
        gap: '12px',
        marginBottom: '20px',
      })}
    >
      <button
        class={css({
          paddingX: '16px',
          paddingY: '12px',
          backgroundColor: 'surface.subtle',
          borderWidth: '1px',
          borderStyle: 'solid',
          borderColor: 'border.default',
          borderRadius: '6px',
          cursor: 'pointer',
          textAlign: 'left',
          transition: 'background',
          _hover: {
            backgroundColor: 'surface.muted',
          },
        })}
        onclick={() => {
          onconfirm?.('html');
        }}
        type="button"
      >
        <div
          class={css({
            fontWeight: 'medium',
            fontSize: '14px',
            color: 'text.default',
            marginBottom: '4px',
          })}
        >
          원본 서식 유지
        </div>
        <div
          class={css({
            fontSize: '12px',
            color: 'text.subtle',
          })}
        >
          복사한 텍스트의 서식을 그대로 유지해요.
        </div>
      </button>

      <button
        class={css({
          paddingX: '16px',
          paddingY: '12px',
          backgroundColor: 'surface.subtle',
          borderWidth: '1px',
          borderStyle: 'solid',
          borderColor: 'border.default',
          borderRadius: '6px',
          cursor: 'pointer',
          textAlign: 'left',
          transition: 'background',
          _hover: {
            backgroundColor: 'surface.muted',
          },
        })}
        onclick={() => {
          onconfirm?.('text');
        }}
        type="button"
      >
        <div
          class={css({
            fontWeight: 'medium',
            fontSize: '14px',
            color: 'text.default',
            marginBottom: '4px',
          })}
        >
          문서 서식 적용
        </div>
        <div
          class={css({
            fontSize: '12px',
            color: 'text.subtle',
          })}
        >
          현재 문서의 서식을 적용하여 붙여넣어요.
        </div>
      </button>
    </div>

    <div
      class={flex({
        justifyContent: 'flex-end',
      })}
    >
      <Button
        onclick={() => {
          open = false;
        }}
        size="sm"
        variant="secondary"
      >
        취소
      </Button>
    </div>
  </div>
</Modal>
