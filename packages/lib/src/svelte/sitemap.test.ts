import assert from 'node:assert/strict';
import test from 'node:test';
import { sitemap, sitemapIndex } from './sitemap.ts';
import type { RequestEvent } from '@sveltejs/kit';

const event = { url: new URL('https://typie.me/sitemap.xml') } as RequestEvent;

test('sitemap wraps paths in a urlset under the request origin', async () => {
  const body = await sitemap(event, ['/', '/@my-space']).text();
  assert.match(body, /<urlset xmlns="http:\/\/www\.sitemaps\.org\/schemas\/sitemap\/0\.9">/);
  assert.match(body, /<loc>https:\/\/typie\.me\/@my-space<\/loc>/);
});

test('sitemapIndex wraps paths in a sitemapindex with sitemap entries', async () => {
  const response = sitemapIndex(event, ['/sitemap-root.xml', '/@my-space/sitemap.xml']);
  assert.equal(response.headers.get('Content-Type'), 'application/xml');
  const body = await response.text();
  assert.match(body, /<sitemapindex xmlns="http:\/\/www\.sitemaps\.org\/schemas\/sitemap\/0\.9">/);
  assert.match(body, /<sitemap><loc>https:\/\/typie\.me\/sitemap-root\.xml<\/loc><\/sitemap>/);
  assert.match(body, /<sitemap><loc>https:\/\/typie\.me\/@my-space\/sitemap\.xml<\/loc><\/sitemap>/);
  assert.doesNotMatch(body, /<urlset/);
});
