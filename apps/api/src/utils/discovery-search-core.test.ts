import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildPublicationSearchRequest,
  buildSpaceSearchRequest,
  buildTagSearchRequest,
  DISCOVERY_QUERY_MAX_LENGTH,
  DISCOVERY_SEARCH_PUBLICATION_SIZE,
  DISCOVERY_SEARCH_SPACE_SIZE,
  DISCOVERY_SEARCH_TAG_SIZE,
  filterHitsByIds,
  normalizeSearchQuery,
} from './discovery-search-core.ts';

test('search queries are trimmed, cut at the maximum length, and empty when blank', () => {
  assert.equal(DISCOVERY_QUERY_MAX_LENGTH, 200);
  assert.equal(normalizeSearchQuery('  바다  '), '바다');
  assert.equal(normalizeSearchQuery(' '.repeat(3)), '');
  assert.equal(normalizeSearchQuery('가'.repeat(250)).length, 200);
});

test('publication search matches title, subtitle, tags and text with boosts, filters discoverable, and highlights', () => {
  const request = buildPublicationSearchRequest({ index: 'dev-publications', query: '바다', decomposedQuery: 'ㅂㅏㄷㅏ' });
  assert.equal(request.index, 'dev-publications');
  assert.equal(request.size, DISCOVERY_SEARCH_PUBLICATION_SIZE);
  assert.equal(DISCOVERY_SEARCH_PUBLICATION_SIZE, 20);
  assert.equal(request._source, false);
  assert.deepEqual(request.query.bool.filter, [{ term: { discoverable: true } }]);
  assert.equal(request.query.bool.minimum_should_match, 1);
  assert.deepEqual(request.query.bool.should, [
    { match: { title: { query: '바다', boost: 3 } } },
    { match: { subtitle: { query: '바다', boost: 2 } } },
    { match: { tags_text: { query: '바다', boost: 2 } } },
    { match: { text: { query: '바다' } } },
    { match: { title_decomposed: { query: 'ㅂㅏㄷㅏ', boost: 1.5 } } },
    { match: { subtitle_decomposed: { query: 'ㅂㅏㄷㅏ', boost: 1 } } },
  ]);
  assert.deepEqual(Object.keys(request.highlight.fields), ['title', 'subtitle', 'text']);
  assert.deepEqual(request.highlight.fields.text, { fragment_size: 200, number_of_fragments: 1 });
  assert.deepEqual(request.highlight.pre_tags, ['<em>']);
  assert.deepEqual(request.highlight.post_tags, ['</em>']);
  assert.equal(request.highlight.encoder, 'html');
});

test('decomposed clauses are omitted when there is no decomposed query', () => {
  const request = buildPublicationSearchRequest({ index: 'dev-publications', query: 'sea', decomposedQuery: null });
  assert.equal(request.query.bool.should.length, 4);
});

test('space search matches name and description and takes three results', () => {
  const request = buildSpaceSearchRequest({ index: 'dev-spaces', query: '바다', decomposedQuery: 'ㅂㅏㄷㅏ' });
  assert.equal(request.size, DISCOVERY_SEARCH_SPACE_SIZE);
  assert.equal(DISCOVERY_SEARCH_SPACE_SIZE, 3);
  assert.deepEqual(request.query.bool.should, [
    { match: { name: { query: '바다', boost: 3 } } },
    { match: { description: { query: '바다' } } },
    { match: { name_decomposed: { query: 'ㅂㅏㄷㅏ', boost: 1.5 } } },
  ]);
  assert.equal('filter' in request.query.bool, false);
});

test('tag search matches the name, sorts by score then count, and takes ten results', () => {
  const request = buildTagSearchRequest({ index: 'dev-tags', query: '바다', decomposedQuery: 'ㅂㅏㄷㅏ' });
  assert.equal(request.size, DISCOVERY_SEARCH_TAG_SIZE);
  assert.equal(DISCOVERY_SEARCH_TAG_SIZE, 10);
  assert.deepEqual(request.query.bool.should, [
    { match: { name: { query: '바다', boost: 2 } } },
    { match: { name_decomposed: { query: 'ㅂㅏㄷㅏ' } } },
  ]);
  assert.deepEqual(request.sort, [{ _score: 'desc' }, { count: 'desc' }]);
});

test('hits are filtered to allowed ids while keeping search order', () => {
  const hits = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
  assert.deepEqual(filterHitsByIds(hits, new Set(['c', 'a'])), [{ id: 'a' }, { id: 'c' }]);
  assert.deepEqual(filterHitsByIds(hits, []), []);
});
