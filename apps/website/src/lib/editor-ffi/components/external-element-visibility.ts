import type { ExternalElement } from '@typie/editor-ffi/browser';

export const shouldKeepEmbedsWhileHidden = (elements: ExternalElement[]): boolean =>
  elements.some((element) => element.data.type === 'embed');

export const visibleExternalElements = (
  overlaysVisible: boolean,
  keepEmbedsWhileHidden: boolean,
  query: () => ExternalElement[],
): ExternalElement[] => {
  if (overlaysVisible) {
    return query();
  }

  if (!keepEmbedsWhileHidden) {
    return [];
  }

  return query().filter((element) => element.data.type === 'embed');
};
