import { SvelteMap } from 'svelte/reactivity';
import { calculateImageSize } from './handlers/image';
import type { ExternalElement, Size } from '@typie/editor-ffi/browser';
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
}
