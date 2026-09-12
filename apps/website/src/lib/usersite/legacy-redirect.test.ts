import { describe, expect, it } from 'vitest';
import { legacyEntityRedirect } from './legacy-redirect';

describe('legacyEntityRedirect', () => {
  it('rewrites a wildcard entity path onto the apex link-share route', () => {
    expect(legacyEntityRedirect({ protocol: 'https:', usersiteHost: 'typie.me', slug: 'abc' })).toBe('https://typie.me/s/abc');
  });

  it('carries the query string over', () => {
    expect(legacyEntityRedirect({ protocol: 'https:', usersiteHost: 'typie.me', slug: 'abc', search: '?x=1' })).toBe(
      'https://typie.me/s/abc?x=1',
    );
  });

  it('leaves the target untouched when there is no query string', () => {
    expect(legacyEntityRedirect({ protocol: 'https:', usersiteHost: 'typie.me', slug: 'abc', search: '' })).toBe('https://typie.me/s/abc');
  });
});
