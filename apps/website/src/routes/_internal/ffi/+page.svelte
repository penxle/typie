<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import Editor from '$lib/editor-ffi/components/Editor.svelte';
  import { setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import type { Doc, Selection } from '@typie/editor-ffi/browser';

  const ctx = setupEditorContext();

  const doc: Doc = {
    nodes: {
      '0': {
        node: { type: 'root' },
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
        children: ['10', '20', '21', '22', '23', '24', '25', '26', '27', '28', '100'],
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
      '100': { node: { type: 'paragraph' }, parent: '0' },
    },
    attrs: {
      layout_mode: {
        type: 'continuous',
        max_width: 400,
      },
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

<div class={center({ position: 'fixed', inset: '0', paddingX: '20px' })}>
  <Editor style={center.raw({ size: 'full' })} {doc} {selection} />
</div>
