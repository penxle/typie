<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import Editor from '$lib/editor-ffi/components/Editor.svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { graphql } from '$mearie';
  import BottomToolbar from './BottomToolbar.svelte';
  import TopToolbar from './TopToolbar.svelte';
  import type { Doc, Selection } from '@typie/editor-ffi/browser';

  const ctx = setupEditorContext();

  const query = createQuery(
    graphql(`
      query FfiPage_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id
          node {
            __typename
            ... on Document {
              id
              ...Editor_document
            }
          }
        }
      }
    `),
    () => ({ entityId: 'E0AAAAAAAAAA' }),
  );

  const document$key = $derived(query.data?.entity?.node.__typename === 'Document' ? query.data.entity.node : null);

  const doc: Doc = {
    nodes: {
      '0': {
        node: {
          type: 'root',
          layout_mode: { type: 'continuous', max_width: 600 },
          // layout_mode: {
          //   type: 'paginated',
          //   page_width: 794,
          //   page_height: 1123,
          //   page_margin_top: 94,
          //   page_margin_bottom: 94,
          //   page_margin_left: 94,
          //   page_margin_right: 94,
          // },
        },
        modifiers: [
          { type: 'font_family', value: 'Pretendard' },
          { type: 'font_weight', value: 400 },
          { type: 'font_size', value: 1200 },
          { type: 'line_height', value: 160 },
          { type: 'letter_spacing', value: 0 },
          { type: 'text_color', value: 'black' },
          { type: 'paragraph_indent', value: 100 },
          { type: 'block_gap', value: 100 },
        ],
        children: ['10', '7', '20', '21', '22', '23', '24', '25', '26', '27', '28', '50', '60', '100'],
        // children: ['7', '20', '21', '100'],
      },
      '10': { node: { type: 'callout', variant: 'danger' }, parent: '0', children: ['1', '3', '5'] },
      '1': { node: { type: 'paragraph' }, parent: '10', children: ['2'] },
      '2': { node: { type: 'text', text: 'A' }, parent: '1' },
      '3': { node: { type: 'paragraph' }, parent: '10', children: ['4'] },
      '4': { node: { type: 'text', text: 'Hello, World!' }, parent: '3' },
      '5': { node: { type: 'paragraph' }, parent: '10', children: ['6'] },
      '6': { node: { type: 'text', text: '안녕하세요!' }, parent: '5' },
      '7': { node: { type: 'paragraph' }, parent: '0' },
      '20': { node: { type: 'horizontal_rule', variant: 'line' }, parent: '0' },
      '21': { node: { type: 'horizontal_rule', variant: 'dashed_line' }, parent: '0' },
      '22': { node: { type: 'horizontal_rule', variant: 'circle' }, parent: '0' },
      '23': { node: { type: 'horizontal_rule', variant: 'three_circles' }, parent: '0' },
      '24': { node: { type: 'horizontal_rule', variant: 'circle_line' }, parent: '0' },
      '25': { node: { type: 'horizontal_rule', variant: 'diamond' }, parent: '0' },
      '26': { node: { type: 'horizontal_rule', variant: 'three_diamonds' }, parent: '0' },
      '27': { node: { type: 'horizontal_rule', variant: 'diamond_line' }, parent: '0' },
      '28': { node: { type: 'horizontal_rule', variant: 'zigzag' }, parent: '0' },
      '50': { node: { type: 'paragraph' }, parent: '0' },
      '60': { node: { type: 'fold' }, parent: '0', children: ['61', '62'] },
      '61': { node: { type: 'fold_title' }, parent: '60', children: ['63'] },
      '62': { node: { type: 'fold_content' }, parent: '60', children: ['64'] },
      '63': { node: { type: 'text', text: '폴드 제목' }, parent: '61' },
      '64': { node: { type: 'paragraph' }, parent: '62', children: ['65'] },
      '65': { node: { type: 'text', text: '폴드 내용!' }, parent: '64' },
      '100': { node: { type: 'paragraph' }, parent: '0' },
    },
  };

  const selection: Selection = {
    anchor: { node_id: '4', offset: 0 },
    head: { node_id: '4', offset: 0 },
  };

  $effect(() => {
    if (ctx.editor?.focusable) {
      ctx.editor.focus();
    }
  });
</script>

<div class={css({ display: 'flex', flexDirection: 'column', size: 'full' })}>
  <TopToolbar />
  <BottomToolbar />

  {#if document$key}
    <Editor style={css.raw({ flex: '1' })} {doc} {document$key} {selection} />
  {/if}
</div>
