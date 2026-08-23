<script lang="ts">
  import { DropdownMenu, DropdownMenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import PyramidIcon from '~icons/lucide/pyramid';
  import { tryMarginContext } from '../../../../routes/website/(dashboard)/[slug]/v2/@prism-review/context.svelte';
  import { roundLabel } from '../../../../routes/website/(dashboard)/[slug]/v2/@prism-review/margin-view';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';

  const app = getAppContext();

  // 미리보기·뷰어 에디터에는 여백 컨텍스트가 없다
  const margin = tryMarginContext();

  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
  const selected = $derived(margin?.rounds.find((round) => round.id === margin.selectedRoundId) ?? null);
</script>

{#if margin && margin.rounds.length > 0}
  <ToolbarDropdownButton active={selected !== null} label="리뷰" placement="bottom-end" size={toolbarSize}>
    {#snippet anchor()}
      <ToolbarIcon icon={PyramidIcon} />
    {/snippet}

    {#snippet floating({ close })}
      <DropdownMenu>
        {#each margin.rounds as round (round.id)}
          <DropdownMenuItem
            active={margin.selectedRoundId === round.id}
            onclick={() => {
              margin.select(round.id);
              close();
            }}
          >
            {roundLabel(round)}
          </DropdownMenuItem>
        {/each}

        <DropdownMenuItem
          active={margin.selectedRoundId === null}
          onclick={() => {
            margin.select(null);
            close();
          }}
        >
          안 봄
        </DropdownMenuItem>
      </DropdownMenu>
    {/snippet}
  </ToolbarDropdownButton>
{/if}
