import { describe, expect, it } from 'vitest';
import { lowercaseSpaceRedirectPath, publicationPath, seriesListPath, seriesPath, spaceHomePath, tagPath } from './paths';

describe('space paths', () => {
  it('builds every path under /@slug', () => {
    expect(spaceHomePath('my-space')).toBe('/@my-space');
    expect(seriesListPath('my-space')).toBe('/@my-space/s');
    expect(seriesPath('my-space', 'c1')).toBe('/@my-space/s/c1');
    expect(publicationPath('my-space', 'p1')).toBe('/@my-space/p/p1');
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
    expect(lowercaseSpaceRedirectPath('/@My%53pace/s', 'MySpace')).toBe('/@myspace/s');
  });

  it('returns null when the lowercased slug is not a valid slug', () => {
    expect(lowercaseSpaceRedirectPath('/@%D0%96x/p/1', 'Жx')).toBeNull();
  });
});
