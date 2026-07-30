<script lang="ts">
  import { setNoteEditsContext } from '$lib/note/note-edit-state.svelte';
  import Note from './Note.svelte';
  import type { NoteEdits } from '$lib/note/note-edit-state.svelte';

  type Props = {
    noteEdits: NoteEdits;
  };

  let { noteEdits }: Props = $props();
  const noop = () => null;

  setNoteEditsContext(noteEdits);
</script>

<Note
  anyDragging={false}
  cancelling={false}
  dragging={false}
  expanded={true}
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
  ondragcancel={noop}
  ondragend={noop}
  ondragmove={noop}
  ondragstart={() => false}
  onexpand={noop}
  onremoveentity={noop}
  ontogglestatus={noop}
  reorderEnabled={false}
  resolving={false}
/>
