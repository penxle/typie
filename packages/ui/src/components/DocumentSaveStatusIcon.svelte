<script lang="ts" module>
  import type { DocumentSaveState } from '@typie/lib/document-save';

  export const documentSaveStatusMessage = (state: DocumentSaveState, protectedChanges = false) => {
    if (state === 'unknown') return '저장 상태를 확인할 수 없어요';
    if (state === 'failed') return '저장하지 못했어요';
    if (state === 'sync-failed')
      return protectedChanges
        ? '최근 변경사항은 이 기기에만 저장되어 있어요.\n서버에 전송하지 못했어요.'
        : '최근 변경사항을 서버에 저장하지 못했어요';
    if (state === 'protected' || (state === 'pending' && protectedChanges))
      return '최근 변경사항은 이 기기에만 저장되어 있어요.\n서버에 전송하는 중이에요.';
    if (state === 'pending') return '저장이 평소보다 오래 걸리고 있어요.\n최근 변경사항을 저장하는 중이에요.';
    return '서버에 저장했어요';
  };
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { fade } from 'svelte/transition';
  import CheckIcon from '~icons/lucide/check';
  import CircleQuestionMarkIcon from '~icons/lucide/circle-question-mark';
  import CloudOffIcon from '~icons/lucide/cloud-off';
  import MonitorCheckIcon from '~icons/lucide/monitor-check';
  import { tooltip } from '../actions/tooltip.svelte';
  import { prefersReducedMotion } from '../state/reduced-motion.svelte';
  import Icon from './Icon.svelte';
  import RingSpinner from './RingSpinner.svelte';

  let {
    status,
    protectedChanges = false,
    showTooltip = true,
    onShowDetails,
  }: {
    status: DocumentSaveState | null;
    protectedChanges?: boolean;
    showTooltip?: boolean;
    onShowDetails?: () => void;
  } = $props();

  const displayedStatus = $derived(protectedChanges && (status === 'pending' || status === 'sync-failed') ? 'protected' : status);
  const style = center({ size: '20px', flexShrink: '0' });
  const dangerStyle = css({ color: 'danger.default' });
  const successStyle = css({ color: 'success.default' });
  const mutedStyle = css({ color: 'text.muted' });
  const colorStyleByState = {
    unknown: mutedStyle,
    pending: mutedStyle,
    failed: dangerStyle,
    'sync-failed': mutedStyle,
    protected: mutedStyle,
    synced: successStyle,
  };
  const iconByState = {
    unknown: CircleQuestionMarkIcon,
    failed: CloudOffIcon,
    'sync-failed': CloudOffIcon,
    protected: MonitorCheckIcon,
    synced: CheckIcon,
  };
</script>

{#if displayedStatus}
  <span class={`${style} ${css({ position: 'relative' })}`} out:fade={{ duration: prefersReducedMotion.current ? 0 : 150 }}>
    {#each [displayedStatus] as state (state)}
      {@const message = documentSaveStatusMessage(status ?? state, protectedChanges)}
      {@const clickable = onShowDetails && (status === 'failed' || status === 'sync-failed')}
      {@const colorStyle = colorStyleByState[state]}
      <span class={css({ position: 'absolute', inset: '0' })} transition:fade={{ duration: prefersReducedMotion.current ? 0 : 150 }}>
        {#snippet glyph()}
          {#if state === 'pending'}
            <RingSpinner style={css.raw({ size: '12px' })} />
          {:else}
            <Icon icon={iconByState[state]} size={12} />
          {/if}
        {/snippet}
        {#if clickable}
          <button
            class={`${style} ${colorStyle} ${css({ cursor: 'pointer', borderRadius: '4px', transition: 'common', _hover: { backgroundColor: 'surface.hover' }, _pressed: { backgroundColor: 'surface.active' } })}`}
            aria-label={status === 'failed' ? '최근 변경사항 저장 상태 확인' : '서버 저장 상태 확인'}
            onclick={onShowDetails}
            type="button"
            use:tooltip={{ message: showTooltip ? message : undefined }}
          >
            {@render glyph()}
          </button>
        {:else}
          <span
            class={`${style} ${colorStyle}`}
            aria-label={state === 'pending' ? '저장 시도 중' : message}
            role="status"
            use:tooltip={{ message: showTooltip ? message : undefined }}
          >
            {@render glyph()}
          </span>
        {/if}
      </span>
    {/each}
  </span>
{/if}
