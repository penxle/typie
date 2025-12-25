<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { CONTINUOUS_PAGE_MARGIN, PAGE_GAP } from '$lib/editor/constants';
  import { getEditor } from '$lib/editor/context';
  import { typewriterPadding } from '$lib/editor/typewriter.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import Cursor from './Cursor.svelte';
  import DocumentPlaceholder from './DocumentPlaceholder.svelte';
  import Input from './Input.svelte';
  import LineHighlight from './LineHighlight.svelte';
  import Page from './Page.svelte';

  type Props = {
    continuousPageMargin?: number;
    contentPadding?: number;
    defaultPaddingBottom?: number;
  };

  let { continuousPageMargin = CONTINUOUS_PAGE_MARGIN, contentPadding = 40, defaultPaddingBottom = 48 }: Props = $props();

  let containerEls = $state<HTMLDivElement[]>([]);
  let inputComponent = $state<Input>();
  let extensionAreaEl = $state<HTMLDivElement>();

  const editor = getEditor();

  $effect(() => {
    editor.inputElement = inputComponent?.getElement() ?? null;
  });

  $effect(() => {
    editor.pageContainerEls = containerEls;
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
    editor.focus();
  };

  const handlePointerDown = (e: PointerEvent) => {
    if (!extensionAreaEl?.contains(e.target as Node)) {
      return;
    }

    editor.handlePointerDown(e);

    // tabindex가 있으면 draggable이어도 drag보다 포커스 이동이 우선됨
    if (editor.isDraggable) {
      extensionAreaEl?.removeAttribute('tabindex');
      setTimeout(() => {
        extensionAreaEl?.setAttribute('tabindex', '0');
      }, 0);
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
  style:padding-left="{contentPadding - continuousPageMargin}px"
  style:padding-right="{contentPadding - continuousPageMargin}px"
  style:gap={isPaginated ? `${PAGE_GAP}px` : '0'}
  class={flex({
    direction: 'column',
    align: 'center',
    grow: '1',
  })}
  aria-label="Editor"
  aria-multiline="true"
  draggable={editor.isDraggable}
  onclick={focusInput}
  ondragend={handleDragEnd}
  ondragenter={handleDragEnter}
  ondragleave={handleDragLeave}
  ondragover={handleDragOver}
  ondragstart={handleDragStart}
  ondrop={handleDrop}
  onfocus={focusInput}
  onfocusin={() => {
    // 클릭 중 input에서 포커스가 벗어나도 커서가 표시될 수 있도록 함
    editor.isFocused = true;
  }}
  onfocusout={(e) => {
    if (editor.inputElement?.contains(e.relatedTarget as Node) || e.relatedTarget === editor.inputElement) {
      return;
    }
    editor.isFocused = false;
  }}
  onkeydown={focusInput}
  role="textbox"
  tabindex="0"
  use:typewriterPadding={defaultPaddingBottom}
>
  {#each editor.layout.pageHeights, i}
    <Page page={i} bind:containerEl={containerEls[i]} />
  {/each}
</div>

<DocumentPlaceholder />
<LineHighlight />
<Cursor />
<Input
  bind:this={inputComponent}
  onBlur={(e) => {
    if (extensionAreaEl?.contains(e.relatedTarget as Node) || e.relatedTarget === extensionAreaEl) {
      return;
    }
    editor.isFocused = false;
  }}
  onFocus={() => {
    editor.isFocused = true;
  }}
/>
<ContextMenu />
