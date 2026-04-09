<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { onDestroy, untrack } from 'svelte';
  import { Editor, getEditorContext } from '../editor.svelte';
  import { handle } from '../handlers';
  import { handlePointerDown, handlePointerMove, handlePointerUp } from '../handlers/pointer';
  import Cursor from './Cursor.svelte';
  import CursorPositioned from './CursorPositioned.svelte';
  import Input from './Input.svelte';
  import Page from './Page.svelte';
  import type { Doc, Selection } from '@typie/editor-ffi/browser';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    doc: Doc;
    selection: Selection;
    style?: SystemStyleObject;
  };

  let { doc, selection, style }: Props = $props();

  const ctx = getEditorContext();

  let status = $state<'uninitialized' | 'initializing' | 'initialized' | 'error'>('uninitialized');
  let clientWidth = $state<number>();
  let clientHeight = $state<number>();

  const init = async (width: number, height: number) => {
    status = 'initializing';
    try {
      ctx.editor = await Editor.create(doc, selection, { width, height, scale_factor: window.devicePixelRatio });
      status = 'initialized';
    } catch (err) {
      console.error(err);
      status = 'error';
    }
  };

  $effect(() => {
    if (status === 'uninitialized' && clientWidth && clientHeight) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      untrack(() => init(clientWidth!, clientHeight!));
    }
  });

  onDestroy(() => {
    ctx.editor?.destroy();
    ctx.editor = undefined;
  });
</script>

<div
  class={css({ position: 'relative', display: 'flex', flexDirection: 'column', alignItems: 'center', userSelect: 'none' }, style)}
  onfocusin={() => ctx.editor?.focus()}
  onfocusout={() => ctx.editor?.blur()}
  onpointerdown={handle(ctx.editor, handlePointerDown)}
  onpointermove={handle(ctx.editor, handlePointerMove)}
  onpointerup={handle(ctx.editor, handlePointerUp)}
  role="textbox"
  tabindex={0}
  bind:clientWidth
  bind:clientHeight
>
  {#if ctx.editor}
    {#each ctx.editor.pageSizes as { width, height }, i (i)}
      <Page {height} page={i} {width} />
    {/each}

    <CursorPositioned>
      <Cursor />
      <Input />
    </CursorPositioned>
  {/if}
</div>
