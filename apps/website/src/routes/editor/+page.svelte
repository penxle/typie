<script lang="ts">
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Toolbar from './Toolbar.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  let editor = $state<Ref<Editor>>();
</script>

<div class={flex({ direction: 'column', alignItems: 'center', gap: '24px', paddingY: '100px', width: 'screen', height: 'screen' })}>
  <!-- {#if editor}
    {JSON.stringify(editor.state.doc.toJSON())}
    {JSON.stringify(editor.state.selection.toJSON())}
  {/if} -->

  {#if editor}
    <Toolbar {editor} />
  {/if}

  <div class={css({ width: 'full', flexGrow: 1 })}>
    <TiptapEditor style={{ height: 'full' }} bind:editor />
  </div>
</div>
