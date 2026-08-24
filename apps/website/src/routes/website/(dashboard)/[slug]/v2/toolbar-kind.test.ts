import { beforeEach, describe, expect, it } from 'vitest';
import { isToolbarKind, otherToolbarKind, primaryToolbarStorageKey, readPrimaryToolbar, writePrimaryToolbar } from './toolbar-kind';

const createStorage = (): Storage => {
  const entries = new Map<string, string>();

  return {
    get length() {
      return entries.size;
    },
    clear: () => {
      entries.clear();
    },
    getItem: (key: string) => entries.get(key) ?? null,
    key: (index: number) => [...entries.keys()][index] ?? null,
    removeItem: (key: string) => {
      entries.delete(key);
    },
    setItem: (key: string, value: string) => {
      entries.set(key, value);
    },
  };
};

Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: createStorage() });

describe('toolbar-kind', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('isToolbarKind는 두 종류만 받아들인다', () => {
    expect(isToolbarKind('insert')).toBe(true);
    expect(isToolbarKind('format')).toBe(true);
    expect(isToolbarKind('classic')).toBe(false);
    expect(isToolbarKind(null)).toBe(false);
  });

  it('otherToolbarKind는 두 종류를 맞바꾼다', () => {
    expect(otherToolbarKind('insert')).toBe('format');
    expect(otherToolbarKind('format')).toBe('insert');
  });

  it('기록이 없으면 null', () => {
    expect(readPrimaryToolbar('doc-1')).toBeNull();
  });

  it('쓴 값을 문서별로 읽는다', () => {
    writePrimaryToolbar('doc-1', 'insert');
    expect(readPrimaryToolbar('doc-1')).toBe('insert');
    expect(readPrimaryToolbar('doc-2')).toBeNull();
    expect(localStorage.getItem(primaryToolbarStorageKey('doc-1'))).toBe('insert');
  });

  it('알 수 없는 값은 기록 없음으로 본다', () => {
    localStorage.setItem(primaryToolbarStorageKey('doc-1'), 'classic');
    expect(readPrimaryToolbar('doc-1')).toBeNull();
  });
});
