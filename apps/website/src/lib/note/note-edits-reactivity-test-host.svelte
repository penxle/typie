<script lang="ts">
  import type { SvelteMap } from 'svelte/reactivity';
  import type { NoteEdits } from './note-edit-state.svelte';

  type Props = {
    noteEdits: NoteEdits;
    noteId: string;
    snapshot: SvelteMap<string, string>;
    onRun: () => void;
  };

  let { noteEdits, noteId, snapshot, onRun }: Props = $props();

  $effect(() => {
    const content = snapshot.get('content') ?? '';
    const color = snapshot.get('color') ?? '';
    noteEdits.sync(noteId, { content, color });
    onRun();
  });
</script>
