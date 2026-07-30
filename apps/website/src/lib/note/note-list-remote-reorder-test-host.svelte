<script lang="ts">
  import NoteList from './NoteList.svelte';
  import type { NoteListState } from './note-list-state.svelte';

  type Note = {
    id: string;
    order: string;
    status: string;
  };

  type Props = {
    state: NoteListState<Note>;
    initialNotes: Note[];
    remoteNotes: Note[];
  };

  let { state: listState, initialNotes, remoteNotes }: Props = $props();
  let authoritativeNotes = $state(initialNotes);
</script>

<button data-test-apply-remote onclick={() => (authoritativeNotes = remoteNotes)} type="button">Apply</button>

<NoteList {authoritativeNotes} identity={{ siteId: 'site-1', status: 'OPEN' }} state={listState}>
  {#snippet children({ item })}
    <div>{item.note.id}</div>
  {/snippet}
</NoteList>
