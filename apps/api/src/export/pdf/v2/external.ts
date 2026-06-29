import type { ExternalElement as EngineExternalElement } from '@typie/editor-ffi/server';
import type { ExternalElement } from '../../core/slate.ts';

export function mapExternalElement(e: EngineExternalElement): ExternalElement {
  const d = e.data;
  const data =
    d.type === 'image'
      ? { type: 'image' as const, id: d.id ?? undefined, proportion: d.proportion / 100 }
      : d.type === 'file'
        ? { type: 'file' as const, id: d.id ?? undefined }
        : d.type === 'embed'
          ? { type: 'embed' as const, id: d.id ?? undefined }
          : { type: 'archived' as const, id: d.id ?? undefined };
  return {
    pageIdx: e.page_idx,
    nodeId: e.node,
    bounds: { x: e.bounds.x, y: e.bounds.y, width: e.bounds.width, height: e.bounds.height },
    data,
  };
}
