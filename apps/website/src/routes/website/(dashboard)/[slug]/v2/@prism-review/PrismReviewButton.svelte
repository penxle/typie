<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ReviewLensIcon from '~icons/typie/review-lens';
  import { tryMarginContext } from './context.svelte';
  import PrismRoundsModal from './PrismRoundsModal.svelte';

  // 미리보기·뷰어 에디터에는 여백 컨텍스트가 없다
  const margin = tryMarginContext();

  let open = $state(false);
</script>

{#if margin && margin.rounds.length > 0}
  {@const active = margin.selectedRoundId !== null}
  <button
    class={center({
      size: '24px',
      flexShrink: '0',
      borderRadius: '4px',
      color: active ? 'accent.brand.default' : 'text.faint',
      transition: 'common',
      _hover: {
        color: active ? 'accent.brand.hover' : 'text.subtle',
        backgroundColor: 'surface.muted',
      },
    })}
    aria-pressed={active}
    onclick={() => (open = true)}
    onpointerdown={(event) => event.preventDefault()}
    type="button"
    use:tooltip={{ message: '리뷰' }}
  >
    <Icon icon={ReviewLensIcon} size={16} />
  </button>
  <PrismRoundsModal bind:open />
{/if}
