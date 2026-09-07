import { describe, expect, it } from 'vitest';
import { resolveLinkShareRedirect } from './link-share-redirect';

describe('resolveLinkShareRedirect', () => {
  it('sends a published document to its publication url', () => {
    expect(resolveLinkShareRedirect({ requestedSlug: 'abc', entitySlug: 'abc', publicationUrl: 'https://s.typie.me/p/PUB0A' })).toBe(
      'https://s.typie.me/p/PUB0A',
    );
  });

  it('canonicalises a redirected slug before anything else', () => {
    expect(resolveLinkShareRedirect({ requestedSlug: 'old', entitySlug: 'new', publicationUrl: null })).toBe('/s/new');
  });

  it('returns null when the view can render in place', () => {
    expect(resolveLinkShareRedirect({ requestedSlug: 'abc', entitySlug: 'abc', publicationUrl: null })).toBeNull();
  });
});
