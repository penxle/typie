<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import TaskView from '../../tasks/[id]/TaskView.svelte';
  import ArtifactsModal from './ArtifactsModal.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let showArtifacts = $state(false);
</script>

<Helmet title="피드백 열람" trailing="타이피 평가" />

{#key data.task.id}
  <TaskView {data} mode="read">
    {#snippet headerAccessory()}
      {#if data.artifacts}
        <button
          class={css({
            paddingX: '10px',
            paddingY: '4px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '6px',
            fontSize: '12px',
            color: 'text.subtle',
            cursor: 'pointer',
            _hover: { color: 'text.default', borderColor: 'border.strong' },
          })}
          onclick={() => (showArtifacts = true)}
          type="button"
        >
          리서치·비평 계획
        </button>
      {/if}
    {/snippet}
  </TaskView>
{/key}

{#if data.artifacts}
  <ArtifactsModal plan={data.artifacts.plan} research={data.artifacts.research} bind:open={showArtifacts} />
{/if}
