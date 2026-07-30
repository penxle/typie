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
    authoritativeNotes: Note[];
  };

  let { state, authoritativeNotes }: Props = $props();
</script>

<NoteList {authoritativeNotes} gap="8px" identity={{ siteId: 'site-1', status: 'OPEN' }} {state}>
  {#snippet children({ item })}
    <div data-test-note-card>{item.note.id}</div>
  {/snippet}
</NoteList>
