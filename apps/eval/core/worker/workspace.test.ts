import { describe, expect, it } from 'vitest';
import { Workspace } from './workspace.ts';
import type { ToolUse } from './agent-loop.ts';

const MANUSCRIPT = 'manuscript/doc-1.txt';
const CONTENT = '홍길동은 문을 열었다. 안내 방송. 다시 안내 방송이었다.';

const workspace = () => {
  const ws = new Workspace([{ path: MANUSCRIPT, content: CONTENT, description: '검토 대상 원고' }]);
  ws.setDeclaredOutputs(['output/plan.yaml']);
  ws.setManuscriptAccess(true);
  return ws;
};

const use = (name: string, input: unknown): ToolUse => ({ id: 't', name, input });

describe('Workspace 권한', () => {
  it('manuscript에는 쓸 수 없다', () => {
    const ws = workspace();
    const out = ws.apply(use('write', { path: MANUSCRIPT, content: 'x' }), 0);
    expect(out?.changed).toBe(false);
    expect(out?.message).toContain('읽기 전용');
  });

  it('output은 선언된 경로만 쓸 수 있다 — 반려에 목록 동봉', () => {
    const ws = workspace();
    const out = ws.apply(use('write', { path: 'output/other.yaml', content: 'x' }), 0);
    expect(out?.changed).toBe(false);
    expect(out?.message).toContain('선언된 산출물');
    expect(out?.message).toContain('output/plan.yaml');
    expect(ws.apply(use('write', { path: 'output/plan.yaml', content: 'axes: []' }), 0)?.changed).toBe(true);
  });

  it('scratch는 자유롭게 쓴다', () => {
    const ws = workspace();
    expect(ws.apply(use('write', { path: 'scratch/notes.md', content: '메모' }), 0)?.changed).toBe(true);
    expect(ws.file('scratch/notes.md')).toBe('메모');
  });

  it('마운트 밖 경로는 쓸 수 없다', () => {
    const ws = workspace();
    expect(ws.apply(use('write', { path: 'notes.md', content: 'x' }), 0)?.changed).toBe(false);
  });
});

describe('Workspace read', () => {
  it('원고는 문자 좌표 창으로 읽고 record를 남긴다', () => {
    const ws = workspace();
    const out = ws.apply(use('read', { path: MANUSCRIPT, start: 0, end: 10 }), 3);
    expect(out?.message).toContain('[0~10]');
    expect(out?.message).toContain(CONTENT.slice(0, 10));
    expect(out?.record).toEqual({ turn: 3, tool: 'read', file: MANUSCRIPT, start: 0, end: 10 });
  });

  it('원고 read에 좌표가 없으면 반려한다', () => {
    const ws = workspace();
    const out = ws.apply(use('read', { path: MANUSCRIPT }), 0);
    expect(out?.message).toContain('start·end 좌표가 필요합니다');
    expect(out?.record).toBeUndefined();
  });

  it('워크스페이스 파일은 줄 번호로 읽고 record가 없다', () => {
    const ws = workspace();
    ws.apply(use('write', { path: 'scratch/notes.md', content: '첫 줄\n둘째 줄' }), 0);
    const out = ws.apply(use('read', { path: 'scratch/notes.md' }), 1);
    expect(out?.message).toContain('1\t첫 줄');
    expect(out?.message).toContain('2\t둘째 줄');
    expect(out?.record).toBeUndefined();
  });

  it('긴 워크스페이스 파일은 잘리고 이어읽기 줄을 안내한다', () => {
    const ws = workspace();
    const lines = Array.from({ length: 400 }, (_, i) => `줄${i + 1} ${'가'.repeat(80)}`).join('\n');
    ws.apply(use('write', { path: 'scratch/long.md', content: lines }), 0);
    const first = ws.apply(use('read', { path: 'scratch/long.md' }), 1);
    expect(first?.message).toContain('이어서 read(start=');
    const at = first?.message.indexOf('이어서 read(start=') ?? 0;
    const next = Number.parseInt((first?.message ?? '').slice(at + '이어서 read(start='.length), 10);
    const second = ws.apply(use('read', { path: 'scratch/long.md', start: next }), 1);
    expect(second?.message).toContain(`${next}\t줄${next}`);
  });

  it('없는 경로는 현재 파일 목록을 동봉해 반려한다', () => {
    const ws = workspace();
    const out = ws.apply(use('read', { path: 'output/none.yaml' }), 0);
    expect(out?.message).toContain('없는 파일');
    expect(out?.message).toContain(MANUSCRIPT);
  });
});

describe('Workspace grep', () => {
  it('원고 grep은 record를 남긴다', () => {
    const ws = workspace();
    const out = ws.apply(use('grep', { path: MANUSCRIPT, pattern: '안내 방송' }), 2);
    expect(out?.message).toContain('총 2건');
    expect(out?.record).toEqual({ turn: 2, tool: 'grep', file: MANUSCRIPT, pattern: '안내 방송', total: 2 });
  });

  it('워크스페이스 파일 grep은 행 번호로 보여주고 record가 없다', () => {
    const ws = workspace();
    ws.apply(use('write', { path: 'output/plan.yaml', content: 'axes:\n  - 리듬' }), 0);
    const out = ws.apply(use('grep', { path: 'output/plan.yaml', pattern: '리듬' }), 1);
    expect(out?.message).toContain('2행');
    expect(out?.record).toBeUndefined();
  });
});

describe('Workspace edit', () => {
  it('정확 일치 한 곳만 바꾼다 — 0곳·여러 곳은 반려', () => {
    const ws = workspace();
    ws.apply(use('write', { path: 'scratch/a.md', content: '하나 둘 하나' }), 0);
    expect(ws.apply(use('edit', { path: 'scratch/a.md', old_string: '없음', new_string: 'x' }), 0)?.changed).toBe(false);
    expect(ws.apply(use('edit', { path: 'scratch/a.md', old_string: '하나', new_string: 'x' }), 0)?.message).toContain('2곳에 일치');
    expect(ws.apply(use('edit', { path: 'scratch/a.md', old_string: '둘 하나', new_string: '둘 셋' }), 0)?.changed).toBe(true);
    expect(ws.file('scratch/a.md')).toBe('하나 둘 셋');
  });

  it('치환 문자열의 $ 패턴을 해석하지 않는다', () => {
    const ws = workspace();
    ws.apply(use('write', { path: 'scratch/a.md', content: '값: 없음' }), 0);
    ws.apply(use('edit', { path: 'scratch/a.md', old_string: '없음', new_string: "$' 그대로 $1" }), 0);
    expect(ws.file('scratch/a.md')).toBe("값: $' 그대로 $1");
  });

  it('없는 파일 edit는 write를 안내한다', () => {
    const ws = workspace();
    const out = ws.apply(use('edit', { path: 'output/plan.yaml', old_string: 'x', new_string: 'y' }), 0);
    expect(out?.changed).toBe(false);
    expect(out?.message).toContain('write');
  });
});

describe('Workspace finalize', () => {
  it('확정 파일은 재수정이 반려되고 헤더가 붙는다', () => {
    const ws = workspace();
    ws.apply(use('write', { path: 'output/plan.yaml', content: 'axes: []' }), 0);
    ws.finalize('output/plan.yaml', '# 비평 계획 — 이 파일의 구조:', '비평 계획');
    expect(ws.apply(use('write', { path: 'output/plan.yaml', content: 'x' }), 1)?.message).toContain('확정된 산출물');
    expect(ws.apply(use('edit', { path: 'output/plan.yaml', old_string: 'axes', new_string: 'x' }), 1)?.message).toContain('확정된 산출물');
    expect(ws.file('output/plan.yaml')).toBe('# 비평 계획 — 이 파일의 구조:\naxes: []');
    expect(ws.index()).toContainEqual({ path: 'output/plan.yaml', description: '비평 계획', finalized: true });
  });
});

describe('Workspace 원고 접근 차단', () => {
  it('manuscriptAccess가 꺼지면 원고 read/grep이 반려되고 색인에서 빠진다', () => {
    const ws = workspace();
    ws.setManuscriptAccess(false);
    const read = ws.apply(use('read', { path: MANUSCRIPT, start: 0, end: 10 }), 0);
    expect(read?.message).toContain('원고 접근이 없습니다');
    expect(read?.record).toBeUndefined();
    expect(ws.apply(use('grep', { path: MANUSCRIPT, pattern: 'x' }), 0)?.record).toBeUndefined();
    expect(ws.index().some((f) => f.path === MANUSCRIPT)).toBe(false);
  });
});

describe('Workspace 색인·스냅샷·요약', () => {
  it('index는 경로·설명·확정 여부를 낸다', () => {
    const ws = workspace();
    expect(ws.index()).toContainEqual({ path: MANUSCRIPT, description: '검토 대상 원고', finalized: false });
  });

  it('scratchFiles는 scratch만 4,000자 캡으로 낸다', () => {
    const ws = workspace();
    ws.apply(use('write', { path: 'scratch/long.md', content: '가'.repeat(5000) }), 0);
    ws.apply(use('write', { path: 'output/plan.yaml', content: 'axes: []' }), 0);
    const files = ws.scratchFiles();
    expect(files).toHaveLength(1);
    expect(files[0].path).toBe('scratch/long.md');
    expect(files[0].content.length).toBeLessThanOrEqual(4001);
  });

  it('summarize는 경로를 포함한 한 줄을 낸다', () => {
    const ws = workspace();
    expect(ws.summarize(use('read', { path: MANUSCRIPT, start: 0, end: 10 }))).toContain(MANUSCRIPT);
    expect(ws.summarize(use('write', { path: 'scratch/a.md', content: '메모' }))).toContain('scratch/a.md');
    expect(ws.summarize(use('edit', { path: 'output/plan.yaml', old_string: '줄바꿈\n포함', new_string: 'y' }))).not.toContain('\n');
  });
});

describe('Workspace 결정성', () => {
  it('같은 호출 열을 재적용하면 같은 상태가 된다', () => {
    const uses: [ToolUse, number][] = [
      [use('write', { path: 'scratch/a.md', content: '메모' }), 0],
      [use('write', { path: 'output/plan.yaml', content: 'axes: []' }), 1],
      [use('edit', { path: 'output/plan.yaml', old_string: '[]', new_string: '[리듬]' }), 2],
      [use('read', { path: MANUSCRIPT, start: 0, end: 10 }), 3],
      [use('grep', { path: MANUSCRIPT, pattern: '안내' }), 4],
    ];
    const a = workspace();
    const b = workspace();
    const recordsA = uses.map(([u, t]) => a.apply(u, t)?.record).filter(Boolean);
    const recordsB = uses.map(([u, t]) => b.apply(u, t)?.record).filter(Boolean);
    expect(recordsA).toEqual(recordsB);
    expect(a.file('output/plan.yaml')).toBe(b.file('output/plan.yaml'));
    expect(a.file('scratch/a.md')).toBe(b.file('scratch/a.md'));
  });
});
