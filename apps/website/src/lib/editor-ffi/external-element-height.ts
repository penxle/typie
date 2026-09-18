import type { ExternalElement, ExternalElementHeight } from '@typie/editor-ffi/browser';
import type { EditorContext } from './editor.svelte';

export const EXTERNAL_CARD_HEIGHT = 64;

// Undefined heights need DOM measurement. Keep these decisions aligned with
// the content rendered by the external element components.
export function getExternalElementHeight(ctx: EditorContext, element: ExternalElement): number | undefined {
  const editor = ctx.editor;
  if (!editor) return undefined;

  const data = element.data;
  switch (data.type) {
    case 'image': {
      const asset = data.id ? editor.images.assets.get(data.id) : undefined;
      const upload = editor.images.uploads.get(element.node);
      if (!(asset?.url ?? upload?.url)) return EXTERNAL_CARD_HEIGHT;
      return editor.images.displaySize(element)?.height;
    }
    case 'file': {
      return EXTERNAL_CARD_HEIGHT;
    }
    case 'embed': {
      return data.id && editor.embedAssets.has(data.id) ? undefined : EXTERNAL_CARD_HEIGHT;
    }
    case 'archived': {
      return EXTERNAL_CARD_HEIGHT;
    }
  }
}

export function getExternalElementHeightUpdates(ctx: EditorContext, elements: ExternalElement[]): ExternalElementHeight[] {
  return elements.flatMap((element) => {
    const knownHeight = getExternalElementHeight(ctx, element);
    if (knownHeight === undefined) return [];
    // Layout stores f32 heights; compare in that precision to avoid repeated updates.
    const height = Math.fround(knownHeight);
    return Number.isFinite(height) && height > 0 && height !== element.bounds.height ? [{ node_id: element.node, height }] : [];
  });
}
