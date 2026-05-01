<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import CommitDetailModal from './commit-detail/CommitDetailModal.svelte';
  import GraphSection from './GraphSection.svelte';
  import StateSection from './StateSection.svelte';
  import TimelineSection from './TimelineSection.svelte';
  import type { DebugBus } from './debug-bus.svelte';
  import type { DebugSnapshot } from './types';

  type Props = {
    bus: DebugBus;
    snapshot: DebugSnapshot;
    slug: string;
    open: boolean;
    onClose: () => void;
  };

  let { bus, snapshot, slug, open, onClose }: Props = $props();

  let selectedCommitId = $state<string | null>(null);
  let selectedHashFromState = $state<string | null>(null);
  let loadedCommits = $state<{ id: string; hash: string }[]>([]);

  const resolvedFromHash = $derived(
    selectedHashFromState ? (loadedCommits.find((c) => c.hash === selectedHashFromState)?.id ?? null) : null,
  );

  const effectiveCommitId = $derived(selectedCommitId ?? resolvedFromHash);

  function selectByHash(h: string) {
    selectedHashFromState = h;
    selectedCommitId = null;
  }

  function selectById(id: string) {
    selectedCommitId = id;
    selectedHashFromState = null;
  }

  function clearSelection() {
    selectedCommitId = null;
    selectedHashFromState = null;
  }
</script>

{#if open}
  <aside
    class={css({
      flexGrow: '0',
      flexShrink: '0',
      flexBasis: '420px',
      display: 'flex',
      flexDirection: 'column',
      borderLeftWidth: '1px',
      borderLeftColor: 'border.subtle',
      backgroundColor: 'surface.default',
      fontSize: '11px',
      lineHeight: '[1.45]',
      overflow: 'hidden',
    })}
  >
    <header
      class={css({
        flexGrow: '0',
        flexShrink: '0',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '12px',
        paddingY: '8px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.subtle',
        backgroundColor: 'surface.subtle',
      })}
    >
      <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
        <span
          class={css({
            fontWeight: 'semibold',
            fontSize: '12px',
            letterSpacing: '0.04em',
          })}
        >
          DEBUG
        </span>
      </div>
      <div class={css({ display: 'flex', alignItems: 'center', gap: '8px', color: 'text.muted' })}>
        <button class={css({ cursor: 'pointer' })} aria-label="Close debug panel" onclick={onClose} type="button">✕</button>
      </div>
    </header>

    <div class={css({ flexGrow: '1', flexShrink: '1', display: 'flex', flexDirection: 'column', minHeight: '0', overflow: 'hidden' })}>
      <StateSection onSelectHash={selectByHash} {snapshot} />
      <TimelineSection {bus} onSelectHash={selectByHash} />
      <GraphSection onSelectCommit={selectById} {slug} {snapshot} bind:loadedCommits />
    </div>
  </aside>
{/if}

<CommitDetailModal commitId={effectiveCommitId} onClose={clearSelection} {slug} />
