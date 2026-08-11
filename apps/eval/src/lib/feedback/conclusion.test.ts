import { describe, expect, it } from 'vitest';
import { issueRefsInclude, threadOfIssueRef, threadsOfIssueRefs } from './conclusion.ts';

const thread = (id: string, issueIndex: number, issueId: string | null = null) => ({ id, issueIndex, issueId });

const threads = [thread('s.2.0', 0, 'judgment-1'), thread('s.2.1', 1, 'judgment-2'), thread('s.1.3', 2)];

describe('threadOfIssueRef', () => {
  it('번호 참조는 그 회차 번호 공간(issueIndex)과 대조한다', () => {
    expect(threadOfIssueRef(0, threads)?.id).toBe('s.2.0');
    expect(threadOfIssueRef(2, threads)?.id).toBe('s.1.3');
  });

  it('id 참조는 이슈 신원(issueId)으로 찾는다', () => {
    expect(threadOfIssueRef('judgment-2', threads)?.id).toBe('s.2.1');
  });

  it('id로 못 찾으면 스레드 id로 한 번 더 찾는다', () => {
    expect(threadOfIssueRef('s.1.3', threads)?.id).toBe('s.1.3');
  });

  it('스레드 id와 남의 issueId가 겹치면 issueId가 이긴다', () => {
    const colliding = [thread('dup', 0), thread('s.2.1', 1, 'dup')];
    expect(threadOfIssueRef('dup', colliding)?.id).toBe('s.2.1');
  });

  it('번호 참조는 스레드 id·issueId를 보지 않는다', () => {
    expect(threadOfIssueRef(9, threads)).toBeNull();
  });

  it('가리키는 스레드가 없으면 null이다', () => {
    expect(threadOfIssueRef('judgment-9', threads)).toBeNull();
  });
});

describe('threadsOfIssueRefs', () => {
  it('참조 순서대로 스레드를 편다', () => {
    expect(threadsOfIssueRefs([2, 0], threads).map((t) => t.id)).toEqual(['s.1.3', 's.2.0']);
    expect(threadsOfIssueRefs(['judgment-2', 's.1.3'], threads).map((t) => t.id)).toEqual(['s.2.1', 's.1.3']);
  });

  it('가리키는 스레드가 없는 참조는 접힌다', () => {
    expect(threadsOfIssueRefs([0, 9], threads).map((t) => t.id)).toEqual(['s.2.0']);
    expect(threadsOfIssueRefs(['judgment-9'], threads)).toEqual([]);
  });

  it('같은 스레드를 두 번 가리켜도 한 번만 선다 — each 키가 겹치면 터진다', () => {
    expect(threadsOfIssueRefs([1, 1], threads).map((t) => t.id)).toEqual(['s.2.1']);
    expect(threadsOfIssueRefs(['judgment-1', 's.2.0'], threads).map((t) => t.id)).toEqual(['s.2.0']);
  });
});

describe('issueRefsInclude', () => {
  it('번호 참조는 issueIndex로, id 참조는 issueId·스레드 id로 대조한다', () => {
    expect(issueRefsInclude([0, 1], threads[1])).toBe(true);
    expect(issueRefsInclude([0], threads[1])).toBe(false);
    expect(issueRefsInclude(['judgment-2'], threads[1])).toBe(true);
    expect(issueRefsInclude(['s.1.3'], threads[2])).toBe(true);
    expect(issueRefsInclude(['judgment-1'], threads[1])).toBe(false);
  });

  it('번호 참조는 id를 겨누지 않는다 — 회차가 다른 스레드가 번호로 걸리는 자리는 호출처가 가른다', () => {
    expect(issueRefsInclude([2], threads[2])).toBe(true);
    expect(issueRefsInclude([2], threads[0])).toBe(false);
  });

  it('활성 스레드가 없으면 아무 참조도 포함하지 않는다', () => {
    expect(issueRefsInclude([0], null)).toBe(false);
    expect(issueRefsInclude([], threads[0])).toBe(false);
  });
});
