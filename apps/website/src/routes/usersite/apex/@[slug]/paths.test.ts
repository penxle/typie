import { describe, expect, it } from 'vitest';
import { folderPath, lowercaseSpaceRedirectPath, publicationPath, spaceHomePath, tagPath } from './paths';

describe('space paths', () => {
  it('builds every path under /@slug', () => {
    expect(spaceHomePath('my-space')).toBe('/@my-space');
    expect(folderPath('my-space', '98765432109')).toBe('/@my-space/f/98765432109');
    expect(publicationPath('my-space', '12345678901')).toBe('/@my-space/p/12345678901');
  });

  it('encodes the tag name', () => {
    expect(tagPath('my-space', '한글 태그/슬래시')).toBe(
      '/@my-space/t/%ED%95%9C%EA%B8%80%20%ED%83%9C%EA%B7%B8%2F%EC%8A%AC%EB%9E%98%EC%8B%9C',
    );
  });
});

describe('lowercaseSpaceRedirectPath', () => {
  it('returns null when the slug is already lowercase', () => {
    expect(lowercaseSpaceRedirectPath('/@myspace/p/1', 'myspace')).toBeNull();
  });

  it('lowercases the slug and keeps the rest of the path', () => {
    expect(lowercaseSpaceRedirectPath('/@MySpace/p/1', 'MySpace')).toBe('/@myspace/p/1');
    expect(lowercaseSpaceRedirectPath('/@MySpace', 'MySpace')).toBe('/@myspace');
  });

  it('cuts the percent-encoded slug segment from the pathname', () => {
    expect(lowercaseSpaceRedirectPath('/@My%53pace/f/1', 'MySpace')).toBe('/@myspace/f/1');
  });

  it('returns null when the lowercased slug is not a valid slug', () => {
    expect(lowercaseSpaceRedirectPath('/@%D0%96x/p/1', 'Жx')).toBeNull();
  });
});
