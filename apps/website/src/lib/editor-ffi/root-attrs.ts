import type { LayoutMode, Modifier } from '@typie/editor-ffi/browser';
import type { Editor } from '$lib/editor-ffi/editor.svelte';

const ROOT_ID = '0';

export const defaultPaginatedLayout = (): LayoutMode => ({
  type: 'paginated',
  page_width: 794,
  page_height: 1123,
  page_margin_top: 94,
  page_margin_bottom: 94,
  page_margin_left: 94,
  page_margin_right: 94,
});

export const defaultContinuousLayout = (): LayoutMode => ({ type: 'continuous', max_width: 600 });

export const setRootLayoutMode = (editor: Editor | undefined, layout_mode: LayoutMode) => {
  editor?.enqueue({ type: 'node', op: { type: 'set_attrs', id: ROOT_ID, attrs: { type: 'root', layout_mode } } });
};

export const setRootModifier = (editor: Editor | undefined, modifier: Modifier) => {
  editor?.enqueue({ type: 'modifier', op: { type: 'set_on_node', id: ROOT_ID, modifier } });
};
