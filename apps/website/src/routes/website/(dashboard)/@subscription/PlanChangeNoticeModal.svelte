<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';

  type Props = {
    open: boolean;
    showSubscribe?: boolean;
    onsubscribe?: () => void;
  };

  let { open = $bindable(false), showSubscribe = false, onsubscribe }: Props = $props();

  const description =
    '무료 플랜은 종료되고,\n월 구독료는 2,900원으로 낮아졌어요.\n\n기존 이용자는 7월 27일까지\n모든 기능을 무료로 이용할 수 있어요.';
</script>

<Modal style={css.raw({ alignItems: 'center', paddingX: '20px', paddingY: '28px', maxWidth: '360px' })} bind:open>
  <div class={css({ fontSize: '18px', fontWeight: 'bold', color: 'text.default' })}>구독 플랜 개편 안내</div>

  <div class={css({ marginTop: '12px', fontSize: '13px', color: 'text.muted', textAlign: 'center', whiteSpace: 'pre-line' })}>
    {description}
  </div>

  <div class={flex({ flexDirection: 'column', gap: '8px', marginTop: '24px', width: 'full' })}>
    {#if showSubscribe}
      <Button onclick={() => (open = false)} variant="secondary">좀 더 둘러볼게요</Button>
      <Button
        onclick={() => {
          open = false;
          onsubscribe?.();
        }}
      >
        구독하고 계속 사용하기
      </Button>
    {:else}
      <Button onclick={() => (open = false)}>확인했어요</Button>
    {/if}
  </div>
</Modal>
