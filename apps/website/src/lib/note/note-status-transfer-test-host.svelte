<script lang="ts">
  import { SvelteMap } from 'svelte/reactivity';
  import { NoteActions } from './note-actions.svelte';
  import { NoteListState } from './note-list-state.svelte';

  type Note = {
    id: string;
    order: string;
    status: 'OPEN' | 'RESOLVED';
    updatedAt: string;
  };

  type Props = {
    actions: NoteActions<Note>;
    identity: SvelteMap<string, string>;
    notes: SvelteMap<string, Note>;
  };

  let { actions, identity, notes }: Props = $props();

  const openState = new NoteListState<Note>({ isTerminallyDeleted: () => false });
  const resolvedState = new NoteListState<Note>({ isTerminallyDeleted: () => false });
  const entityId = $derived(identity.get('entityId') ?? '');
  const rawNotes = $derived([...notes.values()]);
  const openNotes = $derived(rawNotes.filter((note) => note.status === 'OPEN' && actions.isStatusAdmitted(note.id, 'OPEN')));
  const resolvedNotes = $derived(rawNotes.filter((note) => note.status === 'RESOLVED' && actions.isStatusAdmitted(note.id, 'RESOLVED')));

  $effect.pre(() => {
    actions.syncStatus({
      siteId: 'site-1',
      entityId,
      notes: rawNotes,
      visibleStatuses: ['OPEN', 'RESOLVED'],
    });
  });

  $effect.pre(() => {
    openState.sync({ siteId: 'site-1', entityId, status: 'OPEN' }, openNotes);
    resolvedState.sync({ siteId: 'site-1', entityId, status: 'RESOLVED' }, resolvedNotes);
  });

  const finishOpenExit = () => {
    openState.finishExiting('note-1');
    actions.finishStatusTransfer('note-1', 'OPEN');
  };
</script>

<div data-list="open">
  {#each openState.visibleNotes() as item (item.note.id)}
    <span data-note-id={item.note.id} data-presence={item.presence}></span>
  {/each}
</div>

<div data-list="resolved">
  {#each resolvedState.visibleNotes() as item (item.note.id)}
    <span data-note-id={item.note.id} data-presence={item.presence}></span>
  {/each}
</div>

<button onclick={finishOpenExit} type="button">finish open exit</button>
