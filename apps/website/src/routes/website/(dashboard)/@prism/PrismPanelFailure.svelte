<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { Button } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import PrismFailureIcon from '~icons/typie/prism-failure';
  import PrismPanelHeader from './PrismPanelHeader.svelte';

  type Props = {
    onRetry: () => void;
  };

  let { onRetry }: Props = $props();

  const app = getAppContext();
  const panelInteractive = $derived(app.preference.current.prismPanelOpen);
  const titleId = 'prism-panel-failure-title';
  const messageId = 'prism-panel-failure-message';
  let retryButton = $state<HTMLElement>();
  let hasFocusedRetry = false;

  $effect(() => {
    if (hasFocusedRetry || !panelInteractive || !retryButton) return;
    hasFocusedRetry = true;
    retryButton.focus({ preventScroll: true });
  });
</script>

<PrismPanelHeader />

<div
  class={flex({
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    flexGrow: '1',
    minHeight: '0',
    padding: '24px',
    textAlign: 'center',
  })}
  aria-describedby={messageId}
  aria-labelledby={titleId}
  data-prism-panel-failure-content
  role="alert"
>
  <div
    style:color={token('colors.border.strong')}
    class={css({ width: '48px', height: '48px', marginBottom: '20px' })}
    aria-hidden="true"
    data-prism-panel-failure-icon
  >
    <PrismFailureIcon class={css({ width: 'full', height: 'full' })} />
  </div>
  <h2 id={titleId} class={css({ fontSize: '20px', fontWeight: 'bold' })}>앗! 문제가 생겼어요</h2>
  <p id={messageId} class={css({ marginTop: '8px', fontSize: '14px', color: 'text.faint' })}>잠시 후 다시 시도해 주세요.</p>
  <Button style={css.raw({ marginTop: '20px' })} onclick={onRetry} size="md" bind:element={retryButton}>다시 시도</Button>
</div>
