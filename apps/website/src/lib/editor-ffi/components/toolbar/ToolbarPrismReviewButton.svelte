<script lang="ts">
  import { getAppContext } from '@typie/ui/context';
  import PrismIcon from '~icons/typie/prism';
  import { tryMarginContext } from '../../../../routes/website/(dashboard)/[slug]/v2/@prism-review/context.svelte';
  import PrismRoundsModal from '../../../../routes/website/(dashboard)/[slug]/v2/@prism-review/PrismRoundsModal.svelte';
  import ToolbarButton from './ToolbarButton.svelte';

  const app = getAppContext();

  // 미리보기·뷰어 에디터에는 여백 컨텍스트가 없다
  const margin = tryMarginContext();

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');

  let open = $state(false);
</script>

{#if margin && margin.rounds.length > 0}
  <ToolbarButton active={margin.selectedRoundId !== null} icon={PrismIcon} label="리뷰" onclick={() => (open = true)} size={toolbarSize} />
  <PrismRoundsModal bind:open />
{/if}
