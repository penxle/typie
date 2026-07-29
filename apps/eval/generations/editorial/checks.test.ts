import { describe, expect, it } from 'vitest';
import { checkEditorialPlan, checkFinding, checkQuote, checkResearch, coverageGaps, mergeVerifications } from './checks.ts';
import type { ToolRecord } from './ledger.ts';
import type { EditorialFinding, EditorialPlan, Research } from './types.ts';

const CONTENT = '머리말입니다. 홍길동이 문을 열었다. 김영희가 고개를 들었다. 안내 방송이 울렸다. 후기: 봐 주신 분들께 고마움을 전합니다.';

const ALL_READ: ToolRecord[] = [{ turn: 0, tool: 'read', start: 0, end: CONTENT.length }];
const PARTIAL_READ: ToolRecord[] = [{ turn: 0, tool: 'read', start: 0, end: 20 }];

describe('checkQuote', () => {
  it('실재하지만 안 읽은 인용은 열람 범위 밖으로 반려한다', () => {
    const r = checkQuote(CONTENT, PARTIAL_READ, '안내 방송이 울렸다');
    expect(r.range).toBeNull();
    expect(r.reason).toContain('열람 범위 밖');
  });

  it('실재하지 않는 인용을 반려한다', () => {
    const r = checkQuote(CONTENT, ALL_READ, '원고에 없는 문장');
    expect(r.reason).toContain('원고에 없음');
  });

  it('읽은 범위의 실재 인용은 좌표를 준다', () => {
    const r = checkQuote(CONTENT, ALL_READ, '문을 열었다');
    expect(r.range).not.toBeNull();
  });
});

const research = (over: Partial<Research>): Research => ({
  nature: { form: '단편', completeness: { level: 'complete', note: '구성 정돈' }, feedbackFit: '구조 지적 유효' },
  voice: { pov: '3인칭 제한', conventions: [{ pattern: '짧은 단문', evidence: ['문을 열었다'] }] },
  names: [],
  premise: { sourceWork: { status: 'not-identified', name: '', brief: '' }, genreConventions: '', seriesContext: '' },
  boundaries: [{ startQuote: '후기: 봐 주신', endQuote: '고마움을 전합니다.', reason: '후기' }],
  unverified: [],
  ...over,
});

describe('checkResearch', () => {
  it('미실재 인용을 삭제하고 boundaries 좌표를 해석한다', () => {
    const r = checkResearch(
      CONTENT,
      research({ voice: { pov: 'p', conventions: [{ pattern: 'x', evidence: ['문을 열었다', '없는 문장'] }] } }),
      ALL_READ,
    );
    expect(r.research.voice.conventions[0].evidence).toEqual(['문을 열었다']);
    expect(r.notes.some((n) => n.includes('원고에 없음'))).toBe(true);
    expect(r.research.boundaryRanges).toHaveLength(1);
    expect(r.research.boundaryRanges[0].start).toBe(CONTENT.indexOf('후기'));
  });

  // 판정 권위는 검색이다 — 검색 기록 없는 특정 주장은 미판정으로 강등한다.
  it('검색 없는 원작 특정을 미판정으로 강등한다', () => {
    const r = checkResearch(
      CONTENT,
      research({
        premise: { sourceWork: { status: 'identified', name: '홍길동전', brief: 'b' }, genreConventions: '', seriesContext: '' },
      }),
      ALL_READ,
    );
    expect(r.research.premise.sourceWork.status).toBe('undetermined');
  });
});

const plan = (over: Partial<EditorialPlan>): EditorialPlan => ({
  intent: 'i',
  protected: [],
  axes: [
    {
      label: '축1',
      inquiry: 'q',
      risk: 'r',
      evidence: ['문을 열었다'],
      conventionsCheck: '규약에 대조할 항목 없음',
      conventionsBasis: 'unrelated',
    },
    {
      label: '축2',
      inquiry: 'q',
      risk: 'r',
      evidence: ['고개를 들었다'],
      conventionsCheck: '규약에 대조할 항목 없음',
      conventionsBasis: 'unrelated',
    },
    {
      label: '축3',
      inquiry: 'q',
      risk: 'r',
      evidence: ['안내 방송이 울렸다'],
      conventionsCheck: '규약에 대조할 항목 없음',
      conventionsBasis: 'unrelated',
    },
  ],
  verifications: [],
  reviewResponses: [],
  ...over,
});

describe('checkEditorialPlan', () => {
  it('축 수의 퇴화 방지선을 판정한다', () => {
    expect(checkEditorialPlan(CONTENT, plan({}), ALL_READ).contractOk).toBe(true);
    expect(checkEditorialPlan(CONTENT, plan({ axes: plan({}).axes.slice(0, 1) }), ALL_READ).contractOk).toBe(true);
    const zero = checkEditorialPlan(CONTENT, plan({ axes: [] }), ALL_READ);
    expect(zero.contractOk).toBe(false);
    expect(zero.notes.some((n) => n.includes('계약'))).toBe(true);
    const sprawl = plan({}).axes[0];
    const many = checkEditorialPlan(
      CONTENT,
      plan({ axes: Array.from({ length: 13 }, (_, i) => ({ ...sprawl, label: `축${i}` })) }),
      ALL_READ,
    );
    expect(many.contractOk).toBe(false);
  });

  it('필드의 도구 구문 누출을 반려한다', () => {
    const leaked = checkEditorialPlan(CONTENT, plan({ intent: '의도</intent><parameter name="protected">[]' }), ALL_READ);
    expect(leaked.contractOk).toBe(false);
    const schemaTag = checkEditorialPlan(CONTENT, plan({ intent: '의도가 여기 있고 </axes> <verifications>[…]' }), ALL_READ);
    expect(schemaTag.contractOk).toBe(false);
    const clean = checkEditorialPlan(
      CONTENT,
      plan({ intent: '접촉이 3<5초의 정적을 만든다는 의도, 그리고 <소리>라는 한글 괄호' }),
      ALL_READ,
    );
    expect(clean.contractOk).toBe(true);
  });

  it('검색했다는 축 신고를 원장으로 반증한다', () => {
    const lied = checkEditorialPlan(
      CONTENT,
      plan({ axes: plan({}).axes.map((a, i) => (i === 0 ? { ...a, conventionsBasis: 'search' as const } : a)) }),
      ALL_READ,
    );
    expect(lied.contractOk).toBe(false);
    expect(lied.notes.some((n) => n.includes('원장에 search가 없음'))).toBe(true);
  });

  it('verifications의 도구 참조가 원장에 없으면 기록한다', () => {
    const r = checkEditorialPlan(
      CONTENT,
      plan({ verifications: [{ question: 'q', tools: ['grep'], detail: "'광역'", conclusion: 'c' }] }),
      ALL_READ,
    );
    expect(r.notes.some((n) => n.includes('원장에 없음'))).toBe(true);
  });

  it('근거 전멸 보호는 취소하되 축은 남긴다', () => {
    const r = checkEditorialPlan(
      CONTENT,
      plan({
        protected: [{ technique: '허구', evidence: ['없는 문장'], rationale: 'r' }],
        axes: plan({}).axes.map((a, i) => (i === 0 ? { ...a, evidence: ['없는 문장'] } : a)),
      }),
      ALL_READ,
    );
    expect(r.plan.protected).toEqual([]);
    expect(r.plan.axes).toHaveLength(3);
    expect(r.plan.axes[0].evidence).toEqual([]);
  });
});

const finding = (over: Partial<EditorialFinding>): EditorialFinding => ({
  axis: '축1',
  quoteStart: '문을 열었다',
  quoteEnd: '문을 열었다',
  intent: 'i',
  observation: 'o',
  cause: 'c',
  direction: 'd',
  evidence: 'e',
  manuscriptBasis: 'local',
  manuscriptCheck: '단일 지점 지적 — 전문 대조 불요',
  conventionsCheck: '규약의 문체 관습에 해당 항목 없음',
  ...over,
});

describe('checkFinding', () => {
  const AXES = ['축1', '축2'];
  const BOUNDS = [{ start: CONTENT.indexOf('후기'), end: CONTENT.length }];

  it('정상 제출은 좌표·턴과 함께 접수한다', () => {
    const r = checkFinding(CONTENT, finding({}), ALL_READ, 5, AXES, BOUNDS);
    expect(r.accepted).not.toBeNull();
    expect(r.accepted?.filedAtTurn).toBe(5);
  });

  it('인용부 밖 좌표 나열을 반려하되 작품 수치 인용은 통과시킨다', () => {
    const leaked = checkFinding(CONTENT, finding({ evidence: "1851 '안내 방송', 3216 '안내 문구'" }), ALL_READ, 0, AXES, BOUNDS);
    expect(leaked.reasons.some((x) => x.includes('문자 좌표'))).toBe(true);
    const legit = checkFinding(
      CONTENT,
      finding({ evidence: "'반경 6100미터. 9300미터.'라는 수치 상승 인용과 '2018년'이라는 연도" }),
      ALL_READ,
      0,
      AXES,
      BOUNDS,
    );
    expect(legit.reasons.some((x) => x.includes('문자 좌표'))).toBe(false);
  });

  it('지적 필드의 도구 구문 누출을 반려한다', () => {
    const r = checkFinding(CONTENT, finding({ cause: '원인<parameter name="direction">' }), ALL_READ, 0, AXES, BOUNDS);
    expect(r.reasons.some((x) => x.includes('도구 호출 구문'))).toBe(true);
  });

  it('계획에 없는 축을 반려한다', () => {
    expect(checkFinding(CONTENT, finding({ axis: '없는축' }), ALL_READ, 0, AXES, BOUNDS).reasons[0]).toContain('계획에 없음');
  });

  it('열람 범위 밖 앵커를 반려한다', () => {
    const r = checkFinding(
      CONTENT,
      finding({ quoteStart: '안내 방송이 울렸다', quoteEnd: '안내 방송이 울렸다' }),
      PARTIAL_READ,
      0,
      AXES,
      BOUNDS,
    );
    expect(r.reasons.some((x) => x.includes('열람 범위 밖'))).toBe(true);
  });

  it('제외 구간 앵커를 반려한다', () => {
    const r = checkFinding(CONTENT, finding({ quoteStart: '후기: 봐 주신', quoteEnd: '고마움을 전합니다.' }), ALL_READ, 0, AXES, BOUNDS);
    expect(r.reasons.some((x) => x.includes('제외 구간'))).toBe(true);
  });

  it('대조했다는데 grep 기록이 없으면 반려한다', () => {
    const r = checkFinding(CONTENT, finding({ manuscriptBasis: 'grep' }), ALL_READ, 0, AXES, BOUNDS);
    expect(r.reasons.some((x) => x.includes('grep 기록이 없음'))).toBe(true);
  });
});

describe('mergeVerifications', () => {
  it('이전 확정을 보존하고 같은 질문은 최신본이 이긴다', () => {
    const prev = [
      { question: 'q1', tools: ['search' as const], detail: 'a', conclusion: 'old' },
      { question: 'q2', tools: ['grep' as const], detail: 'b', conclusion: 'kept' },
    ];
    const next = [{ question: 'q1', tools: ['search' as const], detail: 'a2', conclusion: 'new' }];
    const merged = mergeVerifications(prev, next);
    expect(merged).toHaveLength(2);
    expect(merged.find((v) => v.question === 'q1')?.conclusion).toBe('new');
    expect(merged.find((v) => v.question === 'q2')?.conclusion).toBe('kept');
  });
});

describe('coverageGaps', () => {
  it('제외 구간을 반영해 미열람을 계산한다', () => {
    const gaps = coverageGaps(CONTENT.length, PARTIAL_READ, [{ start: CONTENT.indexOf('후기'), end: CONTENT.length }]);
    expect(gaps).toEqual([{ start: 20, end: CONTENT.indexOf('후기') }]);
  });
});
