import { describe, expect, it } from 'vitest';
import { effectiveResolver, serveVerdict, toolFailure, ToolFailureSchema } from './tools.ts';

describe('effectiveResolver', () => {
  it.each([
    ['search-entities', 'READ_ONLY', 'server'],
    ['search-entities', 'STANDARD', 'server'],
    ['search-entities', 'FULL', 'server'],
    ['list-notes', 'READ_ONLY', 'server'],
    ['read-stats', 'FULL', 'server'],
    ['create-folders', 'READ_ONLY', 'server'],
    ['create-folders', 'STANDARD', 'server'],
    ['create-folders', 'FULL', 'server'],
    ['delete-entities', 'READ_ONLY', 'server'],
    ['delete-entities', 'STANDARD', 'user'],
    ['delete-entities', 'FULL', 'server'],
    ['list-trash', 'READ_ONLY', 'server'],
    ['create-documents', 'READ_ONLY', 'server'],
    ['set-goals', 'STANDARD', 'server'],
    ['delete-notes', 'STANDARD', 'user'],
    ['delete-goals', 'FULL', 'server'],
    ['update-sharing', 'READ_ONLY', 'server'],
  ] as const)('%s × %s → %s', (tool, policy, expected) => {
    expect(effectiveResolver(tool, policy)).toBe(expected);
  });

  it('인터랙티브 user 도구는 정책 무관 user', () => {
    for (const policy of ['READ_ONLY', 'STANDARD', 'FULL'] as const) {
      expect(effectiveResolver('ask-user', policy)).toBe('user');
      expect(effectiveResolver('confirm-review', policy)).toBe('user');
    }
  });

  it('client 도구는 정책 무관 client, 미등재는 user', () => {
    expect(effectiveResolver('list-open-documents', 'FULL')).toBe('client');
    expect(effectiveResolver('unknown-tool', 'FULL')).toBe('user');
  });
});

describe('serveVerdict', () => {
  it.each([
    ['search-entities', 'READ_ONLY', 'execute'],
    ['read-note', 'READ_ONLY', 'execute'],
    ['read-goals', 'STANDARD', 'execute'],
    ['read-sharing', 'FULL', 'execute'],
    ['read-comments', 'READ_ONLY', 'execute'],
    ['list-icons', 'READ_ONLY', 'execute'],
    ['create-folders', 'READ_ONLY', 'deny'],
    ['create-folders', 'STANDARD', 'execute'],
    ['delete-entities', 'READ_ONLY', 'deny'],
    ['delete-entities', 'STANDARD', null],
    ['delete-entities', 'FULL', 'execute'],
    ['create-documents', 'READ_ONLY', 'deny'],
    ['move-entities', 'STANDARD', 'execute'],
    ['delete-notes', 'STANDARD', null],
    ['delete-goals', 'FULL', 'execute'],
    ['update-sharing', 'READ_ONLY', 'deny'],
    ['ask-user', 'FULL', null],
    ['list-open-documents', 'FULL', null],
    ['unknown-tool', 'FULL', null],
  ] as const)('%s × %s → %s', (tool, policy, expected) => {
    expect(serveVerdict(tool, policy)).toBe(expected);
  });
});

describe('toolFailure', () => {
  it('봉투 스키마와 왕복', () => {
    const failure = toolFailure('denied', '지금은 할 수 없어요');
    expect(ToolFailureSchema.parse(failure)).toEqual(failure);
  });
});
