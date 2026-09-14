import assert from 'node:assert/strict';
import test from 'node:test';
import { hashText, TITLE_PAGE_PALETTE, titlePageColors, titlePageCoverVersion } from './title-page.ts';

test('text hash is 32-bit FNV-1a over code points', () => {
  assert.equal(hashText(''), 2_166_136_261);
  assert.equal(hashText('a'), 0xe4_0c_29_2c);
  assert.equal(hashText('에세이'), hashText('에세이'));
});

test('title page colors pick one of the eight palette entries by seed', () => {
  assert.equal(TITLE_PAGE_PALETTE.length, 8);
  for (const entry of TITLE_PAGE_PALETTE) {
    for (const value of Object.values(entry)) assert.match(value, /^#[\da-f]{6}$/);
  }
  const colors = titlePageColors('PUB0A제목');
  assert.ok(TITLE_PAGE_PALETTE.includes(colors));
  assert.equal(titlePageColors('PUB0A제목'), colors);
  assert.equal(colors, TITLE_PAGE_PALETTE[hashText('PUB0A제목') % 8]);
});

test('cover version changes when the title or the space name changes', () => {
  const base = titlePageCoverVersion({ title: '제목', spaceName: '스페이스' });
  assert.equal(titlePageCoverVersion({ title: '제목', spaceName: '스페이스' }), base);
  assert.notEqual(titlePageCoverVersion({ title: '제목2', spaceName: '스페이스' }), base);
  assert.notEqual(titlePageCoverVersion({ title: '제목', spaceName: '스페이스2' }), base);
});
