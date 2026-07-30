<script lang="ts">
  import { setNoteEditsContext } from '$lib/note/note-edit-state.svelte';
  import RelatedNoteItem from '$lib/note/RelatedNoteItem.svelte';
  import type { NoteEdits } from '$lib/note/note-edit-state.svelte';
  import type { RelatedNoteItem_note$key } from '$mearie';

  type Props = {
    noteEdits: NoteEdits;
    onDragStart: () => void;
  };

  let { noteEdits, onDragStart }: Props = $props();
  let dragging = $state(false);
  const noop = () => null;

  setNoteEditsContext(noteEdits);

  const note = {
    id: 'note-1',
    content: 'server content',
    color: 'gray',
    status: 'OPEN',
    entity: { id: 'document-1' },
  } as unknown as RelatedNoteItem_note$key;
</script>

<RelatedNoteItem
  anyDragging={dragging}
  cancelling={false}
  compact
  {dragging}
  note$key={note}
  onAddNote={noop}
  onColorSaved={noop}
  onDelete={noop}
  onDragCancel={noop}
  onDragEnd={() => {
    dragging = false;
  }}
  onDragMove={noop}
  onDragStart={() => {
    dragging = true;
    onDragStart();
    return true;
  }}
  onToggleStatus={noop}
  reorderEnabled
  resolving={false}
  siteId="site-1"
/>
