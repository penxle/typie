<script lang="ts">
  import EditorPages from './components/EditorPages.svelte';
  import type { Editor } from './editor.svelte';
  import type { EditorSurfaceHost } from './editor-surface-host.svelte';

  type Props = {
    editor: Editor;
    createHost: () => EditorSurfaceHost;
  };

  let { editor, createHost }: Props = $props();

  let surfaceHost = $state<EditorSurfaceHost>();

  $effect(() => {
    const host = createHost();
    surfaceHost = host;
    return () => {
      surfaceHost = undefined;
      host.destroy();
    };
  });
</script>

<EditorPages {editor} {surfaceHost} />
