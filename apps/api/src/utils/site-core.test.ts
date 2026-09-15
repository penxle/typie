import assert from 'node:assert/strict';
import test from 'node:test';
import { TypieError } from '@typie/lib/errors';
import { normalizeSiteLinks } from './site-core.ts';

test('site links are trimmed, empty rows dropped, and only http(s) urls pass', () => {
  assert.deepEqual(
    normalizeSiteLinks([
      { label: ' 블로그 ', url: ' https://example.com ' },
      { label: '', url: 'https://x.y' },
    ]),
    [{ label: '블로그', url: 'https://example.com' }],
  );
  assert.throws(
    () => normalizeSiteLinks([{ label: 'a', url: 'javascript:alert(1)' }]),
    (e: unknown) => e instanceof TypieError && e.code === 'site_link_invalid',
  );
  assert.throws(
    () => normalizeSiteLinks([{ label: 'a', url: 'not a url' }]),
    (e: unknown) => e instanceof TypieError && e.code === 'site_link_invalid',
  );
});
