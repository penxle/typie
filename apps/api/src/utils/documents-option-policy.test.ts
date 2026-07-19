import assert from 'node:assert/strict';
import test from 'node:test';
import { EntityVisibility } from '@typie/lib/enums';
import { isPrivateVisibilityOnlyInput } from './documents-option-policy.ts';

test('visibility=PRIVATE 단독 입력은 허용', () => {
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE }), true);
});

test('visibility=UNLISTED는 거부', () => {
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.UNLISTED }), false);
});

test('visibility 누락은 거부', () => {
  assert.equal(isPrivateVisibilityOnlyInput({}), false);
});

test('다른 필드가 함께 오면 거부', () => {
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE, allowReaction: true }), false);
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE, password: 'x' }), false);
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE, thumbnailId: 'IMG1' }), false);
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE, availability: 'UNLISTED' }), false);
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE, contentRating: 'ALL' }), false);
  assert.equal(isPrivateVisibilityOnlyInput({ visibility: EntityVisibility.PRIVATE, protectContent: false }), false);
});
