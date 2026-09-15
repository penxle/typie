import assert from 'node:assert/strict';
import test from 'node:test';
import {
  globalCurrentKey,
  globalDataKey,
  mergeSuggestions,
  normalizeTag,
  prefixesOf,
  queryKey,
  readCount,
  siteCurrentKey,
  siteDataKey,
  TAG_SUGGESTION_LIMIT,
} from './tag-suggest-core.ts';

test('normalizeTag trims, strips leading hashes and applies NFC', () => {
  assert.equal(normalizeTag('  ##여행  '), '여행');
  assert.equal(normalizeTag('#'), '');
  assert.equal(normalizeTag('한글'.normalize('NFD')), '한글');
});

test('queryKey lowercases, disassembles hangul, keeps composing jamo and caps at 20', () => {
  assert.equal(queryKey('바ㄷ'), 'ㅂㅏㄷ');
  assert.equal(queryKey('Hello'), 'hello');
  assert.equal(queryKey(' #여행  일기 '), 'ㅇㅕㅎㅐㅇ ㅇㅣㄹㄱㅣ');
  assert.equal(queryKey(''), '');
  assert.equal(queryKey('가'.repeat(30)).length, 20);
});

test('prefixesOf yields every prefix of the key, of later tokens, and the empty prefix', () => {
  const prefixes = prefixesOf('여행 일기');
  assert.ok(prefixes.includes(''));
  assert.ok(prefixes.includes('ㅇ'));
  assert.ok(prefixes.includes('ㅇㅕㅎㅐㅇ ㅇㅣㄹㄱㅣ'));
  assert.ok(prefixes.includes('ㅇㅣㄹ'));
  assert.ok(!prefixes.includes('ㅕ'));
  assert.equal(new Set(prefixes).size, prefixes.length);
});

test('prefixesOf caps each prefix at 20 characters', () => {
  const prefixes = prefixesOf('가'.repeat(30));
  assert.equal(Math.max(...prefixes.map((p) => p.length)), 20);
  assert.equal(prefixes.length, 21);
});

test('keys are namespaced by scope and build', () => {
  assert.equal(globalCurrentKey(), 'tag-suggest:global:current');
  assert.equal(globalDataKey('b1', 'ㅂㅏ'), 'tag-suggest:global:b1:ㅂㅏ');
  assert.equal(globalDataKey('b1', ''), 'tag-suggest:global:b1:');
  assert.equal(siteCurrentKey('S1'), 'tag-suggest:site:S1:current');
  assert.equal(siteDataKey('S1', 'b2', 'ㅂ'), 'tag-suggest:site:S1:b2:ㅂ');
});

test('readCount leaves room for excluded and overlapping names', () => {
  assert.equal(readCount(0), TAG_SUGGESTION_LIMIT * 2);
  assert.equal(readCount(3), TAG_SUGGESTION_LIMIT * 2 + 3);
});

test('mergeSuggestions drops excluded names, dedupes popular against mine, and caps each section', () => {
  const mine = [
    { name: '봄', count: 3 },
    { name: '여행', count: 2 },
    { name: '일기', count: 1 },
  ];
  const popular = [
    { name: '여행', count: 100 },
    { name: '봄', count: 90 },
    { name: '바다', count: 80 },
    { name: '산', count: 70 },
    { name: '강', count: 60 },
    { name: '호수', count: 50 },
    { name: '섬', count: 40 },
  ];
  const result = mergeSuggestions({ mine, popular, exclude: [' #봄 '] });
  assert.deepEqual(result.mine, [
    { name: '여행', count: 2 },
    { name: '일기', count: 1 },
  ]);
  assert.deepEqual(
    result.popular.map((s) => s.name),
    ['바다', '산', '강', '호수', '섬'],
  );
});

test('mergeSuggestions caps mine at the limit', () => {
  const mine = Array.from({ length: 8 }, (_, i) => ({ name: `t${i}`, count: 8 - i }));
  const result = mergeSuggestions({ mine, popular: [], exclude: [] });
  assert.equal(result.mine.length, TAG_SUGGESTION_LIMIT);
  assert.deepEqual(result.popular, []);
});
