import { describe, expect, it } from 'vitest';
import { deliverableGuide, deliverableTools, finalizeHeader, renderShape, validateOutput } from './deliverable.ts';
import type { Deliverable } from './deliverable.ts';

const SCHEMA = {
  type: 'object',
  properties: {
    intent: { type: 'string', description: '이 글이 하려는 것' },
    axes: {
      type: 'array',
      description: '검토 관점',
      items: {
        type: 'object',
        properties: {
          label: { type: 'string', description: '관점의 이름' },
          basis: { type: 'string', enum: ['charter', 'search'] },
          evidence: { type: 'array', items: { type: 'string' } },
        },
        required: ['label', 'basis', 'evidence'],
      },
    },
  },
  required: ['intent', 'axes'],
};

const PATH = 'output/test.yaml';

const DELIVERABLE: Deliverable = {
  label: '테스트 산출물',
  submitName: 'submit_test',
  submitDescription: '확정한다.',
  outputs: {
    [PATH]: {
      schema: SCHEMA,
      lints: [(content) => (content.includes('오염') ? ['오염이 있습니다'] : [])],
      description: '테스트 산출물 파일',
    },
  },
};

describe('validateOutput', () => {
  it('블록 스칼라의 산문·인용을 이스케이프 없이 담는다', () => {
    const v = validateOutput<{ intent: string }>(DELIVERABLE, PATH, ['intent: |-', '  "따옴표"도 그대로: 콜론도.', 'axes: []'].join('\n'));
    expect(v.notes).toEqual([]);
    expect(v.value?.intent).toBe('"따옴표"도 그대로: 콜론도.');
  });

  it('파싱 오류에 줄 위치를 붙이고 값을 내주지 않는다', () => {
    const v = validateOutput(DELIVERABLE, PATH, ['intent: 좋음', 'axes: [', '  - label: x'].join('\n'));
    expect(v.value).toBeUndefined();
    expect(v.notes.some((n) => /[0-9]+행: YAML 파싱 오류/.test(n))).toBe(true);
  });

  it('스키마 위반을 경로로, lint를 문면으로 짚는다', () => {
    const v = validateOutput(DELIVERABLE, PATH, ['intent: 좋음 오염', 'axes:', '  - basis: charter', '    evidence: []'].join('\n'));
    expect(v.value).toBeUndefined();
    expect(v.notes.some((n) => n.includes('axes[0].label'))).toBe(true);
    expect(v.notes.some((n) => n.includes('오염이 있습니다'))).toBe(true);
  });

  it('파일이 없으면 write를 안내한다', () => {
    const v = validateOutput(DELIVERABLE, PATH, null);
    expect(v.notes[0]).toContain(PATH);
    expect(v.notes[0]).toContain('write');
  });

  it('선언 밖 경로는 선언 목록을 동봉해 반려한다', () => {
    const v = validateOutput(DELIVERABLE, 'output/other.yaml', 'intent: x');
    expect(v.value).toBeUndefined();
    expect(v.notes[0]).toContain(PATH);
  });

  it('산문의 폭 맞춤 개행은 공백으로 접고 빈 줄 문단은 유지한다', () => {
    const v = validateOutput<{ intent: string }>(
      DELIVERABLE,
      PATH,
      ['intent: |-', '  첫 문단의 첫 줄이', '  폭을 맞추며 이어진다.', '', '  둘째 문단이다.', 'axes: []'].join('\n'),
    );
    expect(v.value?.intent).toBe('첫 문단의 첫 줄이 폭을 맞추며 이어진다.\n\n둘째 문단이다.');
  });

  it('verbatim 필드의 개행은 보존한다', () => {
    const deliverable: Deliverable = {
      ...DELIVERABLE,
      outputs: {
        [PATH]: {
          schema: {
            type: 'object',
            properties: { quote: { type: 'string', verbatim: true } },
            required: ['quote'],
          },
          description: '인용 보존 검사',
        },
      },
    };
    const v = validateOutput<{ quote: string }>(deliverable, PATH, ['quote: |-', '  시의 첫 행', '  시의 둘째 행'].join('\n'));
    expect(v.value?.quote).toBe('시의 첫 행\n시의 둘째 행');
  });
});

describe('renderShape', () => {
  it('스키마에서 설명 붙은 YAML 스켈레톤을 렌더한다', () => {
    const shape = renderShape(SCHEMA);
    expect(shape).toContain('intent: 문자열  # 이 글이 하려는 것');
    expect(shape).toContain('axes:  # 검토 관점');
    expect(shape).toContain('- label: 문자열  # 관점의 이름');
    expect(shape).toContain('basis: charter|search');
    expect(shape).toContain('- 문자열');
  });
});

describe('finalizeHeader', () => {
  it('전 줄이 # 주석인 자기 서술 헤더를 렌더한다', () => {
    const header = finalizeHeader(DELIVERABLE.outputs[PATH], DELIVERABLE.label);
    const lines = header.split('\n');
    expect(lines[0]).toContain('테스트 산출물');
    expect(lines.every((l) => l.startsWith('# '))).toBe(true);
    expect(header).toContain('intent');
  });
});

describe('deliverableGuide', () => {
  it('색인·경로별 구조·블록 스칼라 규칙을 렌더한다', () => {
    const guide = deliverableGuide(DELIVERABLE, [
      { path: 'manuscript/doc-1.txt', description: '검토 대상 원고' },
      { path: PATH, description: '' },
    ]);
    expect(guide).toContain('manuscript/doc-1.txt — 검토 대상 원고');
    expect(guide).toContain(`## ${PATH}`);
    expect(guide).toContain('intent: 문자열');
    expect(guide).toContain('|- 블록 스칼라');
    expect(guide).toContain('submit_test');
    expect(guide).toContain('같은 턴에 submit_test까지 함께 호출');
  });
});

describe('deliverableTools', () => {
  it('파일 도구 4종과 경로 제출 도구를 파생한다', () => {
    const tools = deliverableTools(DELIVERABLE);
    expect(tools.map((t) => t.name)).toEqual(['read', 'grep', 'write', 'edit', 'submit_test']);
    const submit = tools[4].input_schema as { properties: Record<string, { description: string }>; required: string[] };
    expect(submit.required).toEqual(['path']);
    expect(submit.properties.path.description).toContain(PATH);
  });
});
