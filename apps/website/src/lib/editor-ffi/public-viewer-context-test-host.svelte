<script lang="ts">
  import LineHighlight from './components/LineHighlight.svelte';
  import { setupEditorContext } from './editor.svelte';
  import { setupEditorScroll } from './scroll.svelte';

  const ctx = setupEditorContext();
  setupEditorScroll(ctx);
</script>

<LineHighlight />
<div data-public-viewer-context-ready></div>
