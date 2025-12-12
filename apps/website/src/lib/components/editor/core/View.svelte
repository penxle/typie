<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { getEditor } from '$lib/editor/context';
  import ContextMenu from './ContextMenu.svelte';
  import Cursor from './Cursor.svelte';
  import Input from './Input.svelte';
  import Page from './Page.svelte';

  type Props = {
    contentPadding?: number;
  };

  let { contentPadding = 48 }: Props = $props();

  let containerEls = $state<HTMLDivElement[]>([]);
  let inputComponent = $state<Input>();

  const editor = getEditor();

  $effect(() => {
    const inputEl = inputComponent?.getElement();
    editor.updateCursorElement(containerEls, inputEl);
  });

  const handlePointerDown = (e: PointerEvent) => {
    editor.handlePointerDown(e);
  };

  const handlePointerMove = (e: PointerEvent) => {
    editor.handlePointerMove(e);
  };

  const handlePointerUp = (e: PointerEvent) => {
    editor.handlePointerUp(e);
  };

  const handleContextMenu = (e: MouseEvent) => {
    editor.handleContextMenu(e);
  };

  const focusInput = () => {
    inputComponent?.focus();
  };

  const handleDragStart = (e: DragEvent) => {
    editor.handleDragStart(e);
  };

  const handleDragOver = (e: DragEvent) => {
    editor.handleDragOver(e);
  };

  const handleDragLeave = (e: DragEvent) => {
    editor.handleDragLeave(e);
  };

  const handleDrop = (e: DragEvent) => {
    editor.handleDrop(e);
  };

  const handleDragEnd = (e: DragEvent) => {
    editor.handleDragEnd(e);
    focusInput();
  };

  const handleDragEnter = (e: DragEvent) => {
    editor.handleDragEnter(e);
  };

  const isPaginated = $derived(editor.layout.layoutMode.type === 'paginated');
</script>

<svelte:window
  oncontextmenu={handleContextMenu}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
/>

<div
  style:padding-top="{contentPadding}px"
  style:padding-bottom="{contentPadding}px"
  style:padding-left="{contentPadding}px"
  style:padding-right="{contentPadding}px"
  class={flex({
    direction: 'column',
    align: 'center',
    minHeight: 'full',
    ...(isPaginated && { backgroundColor: 'surface.muted', gap: '24px' }),
  })}
  draggable={editor.isDraggable}
  ondragend={handleDragEnd}
  ondragenter={handleDragEnter}
  ondragleave={handleDragLeave}
  ondragover={handleDragOver}
  ondragstart={handleDragStart}
  ondrop={handleDrop}
  role="region"
>
  {#each editor.layout.pageHeights, i}
    <Page onCanvasClick={focusInput} page={i} bind:containerEl={containerEls[i]} />
  {/each}
</div>

<Cursor />
<Input bind:this={inputComponent} />
<ContextMenu />
