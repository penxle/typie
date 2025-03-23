<script lang="ts">
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';

  let editor = $state.raw<Editor>();
</script>

<div class={flex({ direction: 'column', alignItems: 'center', paddingY: '100px', width: 'screen', height: 'screen' })}>
  <!-- {#if editor}
    {JSON.stringify(editor.state.doc.toJSON())}
    {JSON.stringify(editor.state.selection.toJSON())}
  {/if} -->

  <div class={css({ width: 'full', flexGrow: 1 })}>
    <TiptapEditor style={{ height: 'full' }} bind:editor />
  </div>
</div>
