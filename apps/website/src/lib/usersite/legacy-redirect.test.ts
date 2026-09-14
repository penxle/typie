import { describe, expect, it } from 'vitest';
import { legacyEntityRedirect, legacyHomeRedirect } from './legacy-redirect';

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

describe('legacyHomeRedirect', () => {
  it('sends a workspace subdomain home to the apex space path', () => {
    expect(legacyHomeRedirect({ protocol: 'https:', usersiteHost: 'typie.me', host: 'lamp.typie.me' })).toBe('https://typie.me/@lamp');
  });

  it('carries the query string over', () => {
    expect(legacyHomeRedirect({ protocol: 'https:', usersiteHost: 'typie.me', host: 'lamp.typie.me', search: '?x=1' })).toBe(
      'https://typie.me/@lamp?x=1',
    );
  });

  it('returns null when the host is not a single-label subdomain of the usersite host', () => {
    expect(legacyHomeRedirect({ protocol: 'https:', usersiteHost: 'typie.me', host: 'typie.me' })).toBeNull();
    expect(legacyHomeRedirect({ protocol: 'https:', usersiteHost: 'typie.me', host: 'a.b.typie.me' })).toBeNull();
    expect(legacyHomeRedirect({ protocol: 'https:', usersiteHost: 'typie.me', host: 'lamp.example.com' })).toBeNull();
  });
});
