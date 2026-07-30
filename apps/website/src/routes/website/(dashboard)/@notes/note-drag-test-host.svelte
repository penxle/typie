<script lang="ts">
  import { setNoteEditsContext } from '$lib/note/note-edit-state.svelte';
  import Note from './Note.svelte';
  import type { NoteEdits } from '$lib/note/note-edit-state.svelte';

  type Props = {
    noteEdits: NoteEdits;
    ondragcancel?: () => void;
    ondragend?: () => void;
    ondragstart: () => void;
  };

  const noop = () => null;
  let { noteEdits, ondragcancel = noop, ondragend = noop, ondragstart }: Props = $props();
  let dragging = $state(false);

  setNoteEditsContext(noteEdits);
</script>

<Note
  anyDragging={dragging}
  cancelling={false}
  {dragging}
  expanded={false}
  note={{
    id: 'note-1',
    content: 'server content',
    color: 'gray',
    status: 'OPEN',
    updatedAt: '2026-07-29T00:00:00.000Z',
    site: { id: 'site-1' },
    entities: [],
  }}
  onColorSaved={noop}
  onaddentity={noop}
  oncollapse={noop}
  ondelete={noop}
  ondragcancel={() => {
    dragging = false;
    ondragcancel();
  }}
  ondragend={() => {
    dragging = false;
    ondragend();
  }}
  ondragmove={noop}
  ondragstart={() => {
    dragging = true;
    ondragstart();
    return true;
  }}
  onexpand={noop}
  onremoveentity={noop}
  ontogglestatus={noop}
  reorderEnabled
  resolving={false}
/>
