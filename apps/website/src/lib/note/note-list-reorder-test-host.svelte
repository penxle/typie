<script lang="ts">
  import NoteList from './NoteList.svelte';
  import type { NoteListIdentity, NoteListState } from './note-list-state.svelte';
  import type { NoteListItemReorder } from './NoteList.svelte';

  type Note = {
    id: string;
    order: string;
    status: string;
  };

  type Props = {
    state: NoteListState<Note>;
    authoritativeNotes: Note[];
    desiredOrder: string[];
    desiredOrders?: string[][];
    membershipChange?: Note[];
    orderChange?: Note[];
    replacement?: {
      identity: NoteListIdentity;
      authoritativeNotes: Note[];
      desiredOrders: string[][];
    };
  };

  let { state: listState, authoritativeNotes, desiredOrder, desiredOrders, membershipChange, orderChange, replacement }: Props = $props();
  let finished = $state(false);
  let useReplacement = $state(false);
  let useMembershipChange = $state(false);
  let useOrderChange = $state(false);
  let desiredOrderIndex = 0;

  const activeIdentity = $derived(useReplacement && replacement ? replacement.identity : { siteId: 'site-1', status: 'OPEN' });
  const activeAuthoritativeNotes = $derived(
    useReplacement && replacement
      ? replacement.authoritativeNotes
      : useMembershipChange && membershipChange
        ? membershipChange
        : useOrderChange && orderChange
          ? orderChange
          : authoritativeNotes,
  );
  const activeDesiredOrders = $derived(useReplacement && replacement ? replacement.desiredOrders : (desiredOrders ?? [desiredOrder]));

  const beginReorder = (reorder: NoteListItemReorder): boolean => {
    if (!reorder.ondragstart()) return false;
    const nextOrder = activeDesiredOrders[Math.min(desiredOrderIndex, activeDesiredOrders.length - 1)];
    if (!nextOrder || !listState.beginReorder(nextOrder)) {
      reorder.ondragcancel();
      return false;
    }
    desiredOrderIndex += 1;
    return true;
  };
</script>

{#if replacement}
  <button
    data-test-use-replacement
    onclick={() => {
      useReplacement = true;
      desiredOrderIndex = 0;
      finished = false;
    }}
    type="button"
  >
    replace
  </button>
{/if}

{#if membershipChange}
  <button data-test-use-membership-change onclick={() => (useMembershipChange = true)} type="button">change membership</button>
{/if}

{#if orderChange}
  <button data-test-use-order-change onclick={() => (useOrderChange = true)} type="button">change order</button>
{/if}

<NoteList authoritativeNotes={activeAuthoritativeNotes} identity={activeIdentity} state={listState}>
  {#snippet children({ item, reorder })}
    <button
      data-test-dragging={reorder.dragging ? '' : undefined}
      data-test-reorder={item.note.id}
      onclick={async () => {
        if (!beginReorder(reorder)) return;
        await reorder.ondragend();
        finished = true;
      }}
      type="button"
    >
      {item.note.id}
    </button>
    <button data-test-begin-reorder={item.note.id} onclick={() => beginReorder(reorder)} type="button">begin</button>
  {/snippet}
</NoteList>

{#if finished}
  <span data-test-reconcile-finished></span>
{/if}
