<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';

  type Props = {
    open: boolean;
    onselect?: (editor: 'v1' | 'v2') => void;
  };

  let { open = $bindable(), onselect }: Props = $props();
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
      에디터 선택
    </h2>

    <p
      class={css({
        fontSize: '14px',
        color: 'text.subtle',
        marginBottom: '20px',
        lineHeight: '[1.5]',
      })}
    >
      어떤 에디터로 문서를 작성하시겠어요?
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
          onselect?.('v1');
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
          기존 에디터
        </div>
        <div
          class={css({
            fontSize: '12px',
            color: 'text.subtle',
          })}
        >
          기존 에디터로 포스트를 작성해요.
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
          onselect?.('v2');
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
          v2 에디터
        </div>
        <div
          class={css({
            fontSize: '12px',
            color: 'text.subtle',
          })}
        >
          새로운 에디터로 문서를 작성해요.
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
