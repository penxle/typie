import { describe, expect, it } from 'vitest';
import { generateAbsoluteTasks, generateConfirmationTasks, generateScreeningTasks } from './rounds.ts';

const seq = (values: number[]) => {
  let i = 0;
  return () => values[i++ % values.length];
};

const docs = Array.from({ length: 10 }, (_, i) => ({
  documentId: `doc${i}`,
  setIds: [`s${i}a`, `s${i}b`, `s${i}c`],
}));

describe('generateScreeningTasks', () => {
  it('문서당 ranking 태스크 1개를 만들고 setIds를 보존한다', () => {
    const tasks = generateScreeningTasks(docs, { overlapRatio: 0, rng: seq([0.9]) });
    const rankings = tasks.filter((t) => t.kind === 'ranking');
    expect(rankings).toHaveLength(10);
    for (const [i, task] of rankings.entries()) {
      expect(task.documentId).toBe(`doc${i}`);
      expect([...task.setIds].toSorted((a, b) => a.localeCompare(b))).toEqual(
        [`s${i}a`, `s${i}b`, `s${i}c`].toSorted((a, b) => a.localeCompare(b)),
      );
      expect(task.golden).toBe(false);
    }
  });

  it('overlapRatio에 따라 requiredJudgments 2가 배정된다', () => {
    const tasks = generateScreeningTasks(docs, { overlapRatio: 0.2, rng: seq([0.1, 0.9]) });
    const overlapped = tasks.filter((t) => t.requiredJudgments === 2);
    const single = tasks.filter((t) => t.requiredJudgments === 1);
    expect(overlapped.length + single.length).toBe(10);
    expect(overlapped.length).toBe(5);
  });

  it('rng가 다르면 셔플 순서가 달라질 수 있다', () => {
    const a = generateScreeningTasks(docs, { overlapRatio: 0, rng: seq([0.01, 0.99, 0.5]) });
    const b = generateScreeningTasks(docs, { overlapRatio: 0, rng: seq([0.99, 0.01, 0.5]) });
    const orders = (tasks: typeof a) => tasks.map((t) => t.setIds.join(','));
    expect(orders(a)).not.toEqual(orders(b));
  });
});

describe('generateConfirmationTasks', () => {
  it('문서당 pair 태스크 1개, 전원 평가(requiredJudgments null), golden', () => {
    const input = [
      { documentId: 'doc0', v0SetId: 'v0-0', candidateSetId: 'c-0' },
      { documentId: 'doc1', v0SetId: 'v0-1', candidateSetId: 'c-1' },
    ];
    const tasks = generateConfirmationTasks(input, { rng: seq([0.1, 0.9]) });
    expect(tasks).toHaveLength(2);
    for (const [i, task] of tasks.entries()) {
      expect(task.kind).toBe('pair');
      expect(task.requiredJudgments).toBeNull();
      expect(task.golden).toBe(true);
      expect([...task.setIds].toSorted((a, b) => a.localeCompare(b))).toEqual([`c-${i}`, `v0-${i}`].toSorted((a, b) => a.localeCompare(b)));
    }
    expect(tasks[0].setIds).toEqual(['v0-0', 'c-0']);
    expect(tasks[1].setIds).toEqual(['c-1', 'v0-1']);
  });

  it('A/B 배치가 rng로 랜덤화된다', () => {
    const input = [{ documentId: 'doc0', v0SetId: 'v0', candidateSetId: 'cand' }];
    const front = generateConfirmationTasks(input, { rng: seq([0.1]) });
    const back = generateConfirmationTasks(input, { rng: seq([0.9]) });
    expect(front[0].setIds).not.toEqual(back[0].setIds);
  });
});

describe('generateAbsoluteTasks', () => {
  const input = [
    { documentId: 'doc0', setId: 's0' },
    { documentId: 'doc1', setId: 's1' },
  ];

  it('문서당 세트 1벌짜리 ranking 태스크를 만든다', () => {
    const tasks = generateAbsoluteTasks(input, { requiredJudgments: 2, overlapRatio: 0, rng: seq([0.9]) });
    expect(tasks).toHaveLength(2);
    for (const [i, task] of tasks.entries()) {
      expect(task.kind).toBe('ranking');
      expect(task.documentId).toBe(`doc${i}`);
      expect(task.setIds).toEqual([`s${i}`]);
      expect(task.golden).toBe(false);
    }
  });

  it('중복 구간이 아니면 지정한 판정 수만큼만 배정한다', () => {
    const tasks = generateAbsoluteTasks(input, { requiredJudgments: 3, overlapRatio: 0, rng: seq([0.9]) });
    expect(tasks.map((t) => t.requiredJudgments)).toEqual([3, 3]);
  });

  it('중복 문서는 상한 없이 열어 전원이 본다', () => {
    const tasks = generateAbsoluteTasks(input, { requiredJudgments: 2, overlapRatio: 0.5, rng: seq([0.1]) });
    expect(tasks.filter((t) => t.requiredJudgments === null)).toHaveLength(1);
    expect(tasks.filter((t) => t.requiredJudgments === 2)).toHaveLength(1);
  });

  it('중복 비율 1이면 모든 문서를 전원이 본다', () => {
    const tasks = generateAbsoluteTasks(input, { requiredJudgments: 2, overlapRatio: 1, rng: seq([0.99]) });
    expect(tasks.map((t) => t.requiredJudgments)).toEqual([null, null]);
  });

  it('중복 문서 수가 비율대로 정확히 나온다 — 주사위를 굴리지 않는다', () => {
    const many = Array.from({ length: 30 }, (_, i) => ({ documentId: `doc${i}`, setId: `s${i}` }));
    for (const ratio of [0, 0.07, 0.1, 0.5]) {
      const tasks = generateAbsoluteTasks(many, { requiredJudgments: 1, overlapRatio: ratio, rng: seq([0.3, 0.7, 0.1, 0.9]) });
      expect(tasks.filter((t) => t.requiredJudgments === null)).toHaveLength(Math.round(ratio * 30));
    }
  });
});
