<script lang="ts">
  import Page from './components/Page.svelte';
  import { setupEditorContext } from './editor.svelte';
  import type { Editor } from './editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const ctx = setupEditorContext();
  ctx.editor = editor;
</script>

<div
  {@attach (root) => {
    editor.scrollRootEl = root;

    return () => {
      if (editor.scrollRootEl === root) editor.scrollRootEl = undefined;
    };
  }}
  data-page-lifecycle-scroll-root
>
  <Page backingHeight={400} height={300} page={0} width={200} />
</div>
