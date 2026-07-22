import { describe, expect, it } from 'vitest';
import { effectiveContributions } from './contributions.ts';
import type { ContributionJudgment } from './contributions.ts';

const judgment = (id: string, taskId: string, email: string, at: number): ContributionJudgment => ({
  id,
  taskId,
  evaluatorEmail: email,
  createdAt: new Date(at * 1000),
});

describe('effectiveContributions', () => {
  it('필요 수 안에 든 선착 판정만 센다', () => {
    const tasks = [
      { id: 't1', requiredJudgments: 1 },
      { id: 't2', requiredJudgments: 2 },
    ];
    const judgments = [
      judgment('j1', 't1', 'a@x', 1),
      judgment('j2', 't1', 'b@x', 2),
      judgment('j3', 't1', 'c@x', 3),
      judgment('j4', 't2', 'b@x', 1),
      judgment('j5', 't2', 'c@x', 2),
    ];
    const counts = effectiveContributions(tasks, judgments);
    expect(counts.get('a@x')).toBe(1);
    expect(counts.get('b@x')).toBe(2);
    expect(counts.get('c@x')).toBe(1);
  });

  it('동시각은 id로 결정적 타이브레이크', () => {
    const tasks = [{ id: 't1', requiredJudgments: 1 }];
    const judgments = [judgment('b', 't1', 'late@x', 1), judgment('a', 't1', 'first@x', 1)];
    const counts = effectiveContributions(tasks, judgments);
    expect(counts.get('first@x')).toBe(1);
    expect(counts.get('late@x')).toBeUndefined();
  });

  it('requiredJudgments가 null이면 1로 취급한다', () => {
    const tasks = [{ id: 't1', requiredJudgments: null }];
    const judgments = [judgment('j1', 't1', 'a@x', 1), judgment('j2', 't1', 'b@x', 2)];
    const counts = effectiveContributions(tasks, judgments);
    expect(counts.get('a@x')).toBe(1);
    expect(counts.get('b@x')).toBeUndefined();
  });
});
