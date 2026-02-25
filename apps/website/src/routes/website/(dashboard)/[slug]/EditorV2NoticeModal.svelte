<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import { browser } from '$app/environment';

  type Props = {
    focused: boolean;
  };

  let { focused }: Props = $props();

  let open = $state(false);
  let hasShown = false;
  const dismissStorageKey = 'editor-v2-notice-dismissed';

  $effect(() => {
    if (focused && !hasShown && !(browser && localStorage.getItem(dismissStorageKey) === 'true')) {
      open = true;
      hasShown = true;
    }
  });

  const handleDismissForever = () => {
    if (browser) {
      localStorage.setItem(dismissStorageKey, 'true');
    }

    open = false;
  };
</script>

<Modal style={css.raw({ maxWidth: '440px' })} bind:open>
  <div class={css({ padding: '24px' })}>
    <h2 class={css({ fontSize: '18px', fontWeight: 'bold', color: 'text.default' })}>새 에디터(v2)가 이제 기본 적용됩니다</h2>

    <div class={flex({ flexDirection: 'column', gap: '8px', marginTop: '12px' })}>
      <p class={css({ fontSize: '14px', lineHeight: '[1.6]', color: 'text.subtle', wordBreak: 'keep-all' })}>
        겉보기는 비슷하지만, 처음부터 완전히 재설계되었습니다.
      </p>
      <p class={css({ fontSize: '14px', lineHeight: '[1.6]', color: 'text.subtle', wordBreak: 'keep-all' })}>
        사용 중 불편한 점이나 원하시는 기능이 있으면 의견을 보내주세요. (에디터 오른쪽 위 '의견 보내기' 버튼)
      </p>
      <p class={css({ fontSize: '14px', lineHeight: '[1.6]', color: 'text.subtle', wordBreak: 'keep-all' })}>
        이전에는 어려웠던 것도 이제는 훨씬 빠르게 반영할 수 있어요.
      </p>
      <p class={css({ fontSize: '14px', lineHeight: '[1.6]', color: 'text.subtle', wordBreak: 'keep-all' })}>
        자세한 변경 내용은 업데이트 노트에서 확인해 주세요.
      </p>
    </div>

    <div class={flex({ justifyContent: 'flex-end', gap: '8px', marginTop: '20px' })}>
      <Button
        external
        href="/changelog"
        onclick={() => {
          open = false;
        }}
        type="link"
        variant="secondary"
      >
        <div class={flex({ alignItems: 'center', gap: '6px' })}>
          <span>업데이트 노트 보기</span>
          <Icon icon={ExternalLinkIcon} size={14} />
        </div>
      </Button>
      <Button
        onclick={() => {
          handleDismissForever();
        }}
        type="button"
      >
        다시 보지 않기
      </Button>
    </div>
  </div>
</Modal>
