<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { onDestroy, untrack } from 'svelte';
  import { initWasm } from '$lib/wasm-ffi.svelte';
  import { graphql } from '$mearie';
  import { PAGE_GAP } from '../constants';
  import { Editor, getEditorContext } from '../editor.svelte';
  import { loadFonts } from '../fonts';
  import { handle } from '../handlers';
  import { handlePointerDown, handlePointerMove, handlePointerUp } from '../handlers/pointer';
  import Caret from './Caret.svelte';
  import CaretPositioned from './CaretPositioned.svelte';
  import Input from './Input.svelte';
  import LineHighlight from './LineHighlight.svelte';
  import Page from './Page.svelte';
  import Scrollbar from './Scrollbar.svelte';
  import type { Doc, Selection } from '@typie/editor-ffi/browser';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Editor_document$key } from '$mearie';

  type Props = {
    document$key: Editor_document$key;
    doc: Doc;
    selection: Selection;
    style?: SystemStyleObject;
  };

  let { document$key, doc, selection, style }: Props = $props();

  const ctx = getEditorContext();

  const document = createFragment(
    graphql(`
      fragment Editor_document on Document {
        id

        fontFamilies(sources: [DEFAULT, USER, FALLBACK]) {
          id
          familyName
          source
          fonts {
            id
            weight
            path
            hash
            chunks
          }
        }
      }
    `),
    () => document$key,
  );

  let status = $state<'uninitialized' | 'initializing' | 'initialized' | 'error'>('uninitialized');
  let clientWidth = $state<number>();
  let clientHeight = $state<number>();

  const isPaginated = $derived(ctx.editor?.rootAttrs?.layout_mode.type === 'paginated');

  const init = async (width: number, height: number) => {
    status = 'initializing';
    try {
      await initWasm();
      loadFonts(document.data.fontFamilies);
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
  style:--page-gap={isPaginated ? `${PAGE_GAP}px` : undefined}
  class={css(
    {
      position: 'relative',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      overflow: 'auto',
      scrollbar: 'hidden',
      userSelect: 'none',
      ...(isPaginated && {
        rowGap: 'var(--page-gap)',
        paddingY: 'var(--page-gap)',
        backgroundColor: 'surface.subtle',
      }),
    },
    style,
  )}
  {@attach (el) => {
    if (!ctx.editor) return;
    ctx.editor.scrollContainerEl = el;
    return () => {
      if (ctx.editor) ctx.editor.scrollContainerEl = undefined;
    };
  }}
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

    <CaretPositioned>
      <Caret />
      <Input />
    </CaretPositioned>

    <LineHighlight />

    <Scrollbar />
  {/if}
</div>
