import assert from 'node:assert/strict';
import test from 'node:test';
import { folderUrl, isUsersiteApexOrigin, parseUsersiteSlug, publicationUrl, siteUrl, usersiteApexUrl } from './usersite-core.ts';

const USERSITE_URL = 'https://*.typie.me';

test('parseUsersiteSlug reads the single subdomain label of a wildcard origin', () => {
  assert.equal(parseUsersiteSlug('https://myspace.typie.me', USERSITE_URL), 'myspace');
});

test('parseUsersiteSlug rejects the apex host, nested labels and foreign hosts', () => {
  assert.equal(parseUsersiteSlug('https://typie.me', USERSITE_URL), null);
  assert.equal(parseUsersiteSlug('https://a.b.typie.me', USERSITE_URL), null);
  assert.equal(parseUsersiteSlug('https://myspace.typie.co', USERSITE_URL), null);
});

test('parseUsersiteSlug rejects an empty label and an origin carrying a port or a path', () => {
  assert.equal(parseUsersiteSlug('https://.typie.me', USERSITE_URL), null);
  assert.equal(parseUsersiteSlug('https://myspace.typie.me:4000', USERSITE_URL), null);
  assert.equal(parseUsersiteSlug('https://myspace.typie.me/path', USERSITE_URL), null);
});

test('isUsersiteApexOrigin accepts the apex origin and rejects wildcard and foreign hosts', () => {
  assert.equal(isUsersiteApexOrigin('https://typie.me', USERSITE_URL), true);
  assert.equal(isUsersiteApexOrigin('https://myspace.typie.me', USERSITE_URL), false);
  assert.equal(isUsersiteApexOrigin('https://typie.co', USERSITE_URL), false);
});

test('usersiteApexUrl strips the wildcard label', () => {
  assert.equal(usersiteApexUrl(USERSITE_URL), 'https://typie.me');
});

test('siteUrl places the slug under the apex origin with an @ prefix', () => {
  assert.equal(siteUrl(USERSITE_URL, 'myspace'), 'https://typie.me/@myspace');
});

test('publication and folder urls hang off the site url', () => {
  assert.equal(publicationUrl(USERSITE_URL, 'myspace', '12345678901'), 'https://typie.me/@myspace/p/12345678901');
  assert.equal(folderUrl(USERSITE_URL, 'myspace', '12345678901'), 'https://typie.me/@myspace/f/12345678901');
});
