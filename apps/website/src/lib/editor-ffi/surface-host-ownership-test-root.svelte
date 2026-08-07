<script lang="ts">
  import { setupEditorContext } from './editor.svelte';
  import SurfaceHostOwnershipTestHost from './surface-host-ownership-test-host.svelte';
  import type { Editor } from './editor.svelte';
  import type { EditorSurfaceHost } from './editor-surface-host.svelte';

  type Props = {
    editor: Editor;
    createHost: () => EditorSurfaceHost;
    onerror: (error: unknown) => void;
  };

  let { editor, createHost, onerror }: Props = $props();

  const ctx = setupEditorContext();

  $effect(() => {
    ctx.editor = editor;
  });
</script>

{#key editor}
  <svelte:boundary {onerror}>
    <SurfaceHostOwnershipTestHost {createHost} {editor} />
  </svelte:boundary>
{/key}
