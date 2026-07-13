import { describe, expect, it, vi } from 'vitest';
import { shouldKeepEmbedsWhileHidden, visibleExternalElements } from './external-element-visibility';
import type { ExternalElement } from '@typie/editor-ffi/browser';

const element = (data: ExternalElement['data'], node = '1@1'): ExternalElement => ({
  page_idx: 0,
  node,
  bounds: { x: 0, y: 0, width: 100, height: 50 },
  is_selected: false,
  data,
});

const embed = element({ type: 'embed', id: 'embed-1' });
const image = element({ type: 'image', id: 'image-1', proportion: 100 }, '2@1');
const file = element({ type: 'file', id: 'file-1' }, '3@1');

describe('visibleExternalElements', () => {
  it('renders every element on a visible page', () => {
    expect(visibleExternalElements(true, false, () => [image, embed, file])).toEqual([image, embed, file]);
  });

  it('renders nothing and skips the query on a hidden page without embeds', () => {
    const query = vi.fn(() => [image, file]);
    expect(visibleExternalElements(false, false, query)).toEqual([]);
    expect(query).not.toHaveBeenCalled();
  });

  it('keeps only embeds mounted on a hidden page with keep-alive', () => {
    expect(visibleExternalElements(false, true, () => [image, embed, file])).toEqual([embed]);
  });

  it('unmounts a kept embed once it is removed from the document', () => {
    expect(visibleExternalElements(false, true, () => [image, file])).toEqual([]);
  });
});

describe('shouldKeepEmbedsWhileHidden', () => {
  it('keeps embeds alive when the page has one', () => {
    expect(shouldKeepEmbedsWhileHidden([image, embed])).toBe(true);
  });

  it('does not keep anything alive without embeds', () => {
    expect(shouldKeepEmbedsWhileHidden([image, file])).toBe(false);
  });

  it('does not keep anything alive on an empty page', () => {
    expect(shouldKeepEmbedsWhileHidden([])).toBe(false);
  });
});
