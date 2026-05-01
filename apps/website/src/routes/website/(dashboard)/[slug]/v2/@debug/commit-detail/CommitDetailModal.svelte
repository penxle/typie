<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Modal, SegmentButtons } from '@typie/ui/components';
  import { graphql } from '$mearie';
  import ConflictsTab from './ConflictsTab.svelte';
  import ObjectsTab from './ObjectsTab.svelte';
  import OverviewTab from './OverviewTab.svelte';
  import StepsTab from './StepsTab.svelte';

  type TabId = 'overview' | 'objects' | 'steps' | 'conflicts';

  type Props = {
    slug: string;
    commitId: string | null;
    onClose: () => void;
  };

  let { slug, commitId, onClose }: Props = $props();

  let stack = $state<string[]>([]);
  let activeTab = $state<TabId>('overview');

  const currentId = $derived(stack.at(-1) ?? null);
  const open = $derived(currentId !== null);

  $effect(() => {
    if (commitId === null) {
      stack = [];
      activeTab = 'overview';
      return;
    }
    if (stack.length === 0 || stack[0] !== commitId) {
      stack = [commitId];
    }
  });

  const query = createQuery(
    graphql(`
      query DocumentEditorV2_Debug_CommitDetail($slug: String!, $id: ID!) {
        document(slug: $slug) {
          id
          commit(id: $id) {
            id
            hash
            objects {
              id
            }
            conflicts {
              id
            }
            ...DocumentEditorV2_Debug_OverviewTab_commit
            ...DocumentEditorV2_Debug_ObjectsTab_commit
            ...DocumentEditorV2_Debug_StepsTab_commit
            ...DocumentEditorV2_Debug_ConflictsTab_commit
          }
        }
      }
    `),
    () => ({ slug, id: currentId ?? '' }),
    () => ({ skip: currentId === null }),
  );

  const c = $derived(query.data?.document.commit?.id === currentId ? query.data.document.commit : null);

  function navigateTo(id: string) {
    stack = [...stack, id];
  }

  function goBack() {
    stack = stack.slice(0, -1);
  }

  const tabItems: { label: string; value: TabId }[] = $derived([
    { label: 'Overview', value: 'overview' },
    { label: `Objects${c ? ` (${c.objects.length})` : ''}`, value: 'objects' },
    { label: 'Steps', value: 'steps' },
    { label: `Conflicts${c ? ` (${c.conflicts.length})` : ''}`, value: 'conflicts' },
  ]);
</script>

<Modal style={css.raw({ overflowY: 'hidden' })} closable loading={open && !c && !query.error} onclose={onClose} {open}>
  <div class={css({ display: 'flex', flexDirection: 'column', height: '[50vh]', minHeight: '[50vh]', overflow: 'hidden' })}>
    <header
      class={css({
        flexGrow: '0',
        flexShrink: '0',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '14px',
        paddingY: '10px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.subtle',
        backgroundColor: 'surface.subtle',
      })}
    >
      <div class={css({ display: 'flex', alignItems: 'center', gap: '10px' })}>
        {#if stack.length > 1}
          <button
            class={css({
              cursor: 'pointer',
              backgroundColor: 'transparent',
              border: 'none',
              padding: '0',
              fontSize: '14px',
              color: 'text.muted',
            })}
            aria-label="Back"
            onclick={goBack}
            type="button"
          >
            ←
          </button>
        {/if}
        <span class={css({ fontFamily: 'mono', fontSize: '12px', fontWeight: 'semibold' })}>
          COMMIT {(c?.hash ?? '').slice(0, 8) || '…'}
        </span>
      </div>
      <button
        class={css({
          cursor: 'pointer',
          backgroundColor: 'transparent',
          border: 'none',
          padding: '0',
          fontSize: '14px',
          color: 'text.muted',
        })}
        aria-label="Close commit detail"
        onclick={onClose}
        type="button"
      >
        ✕
      </button>
    </header>

    <div
      class={css({
        flexGrow: '0',
        flexShrink: '0',
        paddingX: '14px',
        paddingY: '8px',
        borderBottomWidth: '1px',
        borderBottomColor: 'border.subtle',
      })}
    >
      <SegmentButtons items={tabItems} onselect={(v) => (activeTab = v)} size="sm" value={activeTab} />
    </div>

    <div class={css({ flexGrow: '1', flexShrink: '1', minHeight: '0', overflowY: 'auto', padding: '14px' })}>
      {#if query.error}
        <div class={css({ color: 'palette.red', fontSize: '11px' })}>{String(query.error)}</div>
      {:else if c}
        {#if activeTab === 'overview'}
          <OverviewTab commit$key={c} onNavigate={navigateTo} />
        {:else if activeTab === 'objects'}
          <ObjectsTab commit$key={c} />
        {:else if activeTab === 'steps'}
          <StepsTab commit$key={c} />
        {:else if activeTab === 'conflicts'}
          <ConflictsTab commit$key={c} />
        {/if}
      {/if}
    </div>
  </div>
</Modal>
