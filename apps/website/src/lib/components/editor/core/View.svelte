<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { CONTINUOUS_VIEW_PADDING, IS_MAC, PAGE_GAP, PAGINATED_VIEW_PADDING } from '$lib/editor/constants';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import { setupTypewriter } from '$lib/editor/typewriter.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import Cursor from './Cursor.svelte';
  import DocumentPlaceholder from './DocumentPlaceholder.svelte';
  import Input from './Input.svelte';
  import LineHighlight from './LineHighlight.svelte';
  import Page from './Page.svelte';
  import PasteOptions from './PasteOptions.svelte';

  type Props = {
    defaultPaddingBottom?: number;
  };

  let { defaultPaddingBottom = 48 }: Props = $props();

  let containerEls = $state<HTMLDivElement[]>([]);
  let inputComponent = $state<Input>();
  let extensionAreaEl = $state<HTMLDivElement>();

  const { editor } = getEditorContext();

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

  setupTypewriter(() => extensionAreaEl, defaultPaddingBottom);

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
    if (!extensionAreaEl?.contains(e.target as Node)) {
      return;
    }

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
  const viewPadding = $derived(isPaginated ? PAGINATED_VIEW_PADDING : CONTINUOUS_VIEW_PADDING);
</script>

<svelte:window
  oncontextmenu={handleContextMenu}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
/>

<div
  bind:this={extensionAreaEl}
  style:padding-left="{viewPadding}px"
  style:padding-right="{viewPadding}px"
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
  oncopy={(e) => {
    if (!editor.readOnly) return;
    if (editor.protectContent) {
      e.preventDefault();
      return;
    }
    const data = editor.getClipboardData();
    if (data) {
      e.preventDefault();
      e.clipboardData?.setData('text/html', data.html);
      e.clipboardData?.setData('text/plain', data.text);
    }
  }}
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
  onkeydown={(e) => {
    if (editor.readOnly) {
      const cmdKey = IS_MAC ? e.metaKey : e.ctrlKey;
      const key = e.key.toLowerCase();
      if (cmdKey && key === 'a') {
        e.preventDefault();
        editor.dispatch({ type: 'selectAll' }).scrollIntoView();
      } else if (cmdKey && key === 'c' && editor.protectContent) {
        e.preventDefault();
      }
      return;
    }
    focusInput();
  }}
  role="textbox"
  tabindex="0"
>
  {#each editor.layout.pages, i}
    <Page page={i} bind:containerEl={containerEls[i]} />
  {/each}
</div>

{#if !editor.readOnly}
  <DocumentPlaceholder />
  <LineHighlight />
  <Cursor />
  <PasteOptions />
{/if}

{#if !editor.readOnly}
  <Input
    bind:this={inputComponent}
    onBlur={(e) => {
      if (editor.pointer.isPressed) {
        return;
      }
      if (extensionAreaEl?.contains(e.relatedTarget as Node) || e.relatedTarget === extensionAreaEl) {
        return;
      }
      editor.isFocused = false;
    }}
    onFocus={() => {
      editor.isFocused = true;
    }}
  />
{/if}

<ContextMenu />
