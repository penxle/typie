import { SvelteMap } from 'svelte/reactivity';
import { calculateImageSize } from './handlers/image';
import type { ExternalElement, ExternalElementHeight, Size } from '@typie/editor-ffi/browser';
import type { ImageAsset } from './types';

export class EditorExternalImageElementState {
  readonly assets = new SvelteMap<string, ImageAsset>();
  readonly uploads = new SvelteMap<string, { uploadId: string; url?: string; width: number; height: number }>();
  readonly resizeDrafts = new SvelteMap<string, number>();

  displaySize(element: ExternalElement): Size | undefined {
    if (element.data.type !== 'image') return undefined;
    const asset = (element.data.id ? this.assets.get(element.data.id) : undefined) ?? this.uploads.get(element.node);
    if (!asset) return undefined;
    return calculateImageSize({
      boundsWidth: element.bounds.width,
      proportion: this.resizeDrafts.get(element.node) ?? element.data.proportion,
      originalWidth: asset.width,
      originalHeight: asset.height,
      maxHeight: element.data.max_height,
    });
  }

  heightUpdates(elements: ExternalElement[]): ExternalElementHeight[] {
    return elements.flatMap((element) => {
      const size = this.displaySize(element);
      if (!size) return [];
      // Layout stores f32 heights; compare in that precision to avoid repeated updates.
      const height = Math.fround(size.height);
      return Number.isFinite(height) && height > 0 && height !== element.bounds.height ? [{ node_id: element.node, height }] : [];
    });
  }
}
