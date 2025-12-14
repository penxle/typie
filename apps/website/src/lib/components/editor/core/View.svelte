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
  let extensionAreaEl = $state<HTMLDivElement>();

  const editor = getEditor();

  $effect(() => {
    const inputEl = inputComponent?.getElement();
    editor.updateCursorElement(containerEls, inputEl);
  });

  $effect(() => {
    if (extensionAreaEl) {
      editor.extensionArea.containerEl = extensionAreaEl;
    }
  });

  $effect(() => {
    editor.extensionArea.pageElements = containerEls.filter((el): el is HTMLDivElement => el != null);
  });

  const focusInput = () => {
    inputComponent?.focus();
  };

  const handlePointerDown = (e: PointerEvent) => {
    if (extensionAreaEl?.contains(e.target as Node)) {
      e.preventDefault();
      focusInput();
      editor.handlePointerDown(e);
    }
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
  bind:this={extensionAreaEl}
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
  role="application"
>
  {#each editor.layout.pageHeights, i}
    <Page page={i} bind:containerEl={containerEls[i]} />
  {/each}
</div>

<Cursor />
<Input bind:this={inputComponent} />
<ContextMenu />
