import { describe, expect, it } from 'vitest';
import {
  renderEditorialComposeReviewInput,
  renderPlanForReview,
  renderRejection,
  renderResearchCharter,
  renderToolTrail,
} from './render.ts';
import type { EditorialPlan, ResolvedResearch } from './types.ts';

const RESEARCH: ResolvedResearch = {
  nature: { form: '단편', completeness: { level: 'complete', note: '완성고' }, feedbackFit: '구조 지적 유효' },
  voice: { pov: '3인칭 제한', conventions: [{ pattern: '짧은 단문', evidence: ['문을 열었다'] }] },
  names: [{ name: '홍길동', aliases: ['길동이'], note: '' }],
  premise: { sourceWork: { status: 'not-identified', name: '', brief: '' }, genreConventions: '', seriesContext: '' },
  boundaries: [],
  boundaryRanges: [],
  unverified: ['화자의 회상 시점'],
};

describe('renderResearchCharter', () => {
  it('여섯 블록 헤더와 미특정 원작 문구를 담는다', () => {
    const s = renderResearchCharter(RESEARCH);
    for (const header of ['[글의 성격]', '[문체 규약]', '[고유명사]', '[독자 전제]', '[미확인 전제]']) {
      expect(s).toContain(header);
    }
    expect(s).toContain('특정되지 않음(오리지널 취급)');
    expect(s).toContain('길동이');
  });
});

describe('renderToolTrail', () => {
  it('도구 호출 전건을 형식대로 나열한다', () => {
    const s = renderToolTrail([
      { turn: 0, tool: 'read', start: 0, end: 100 },
      { turn: 2, tool: 'grep', pattern: '안내', total: 3 },
      { turn: 4, tool: 'search', query: '홍길동전', hits: 5 },
    ]);
    expect(s).toContain('[턴0] read 0~100');
    expect(s).toContain("[턴2] grep '안내' → 3건");
    expect(s).toContain("[턴4] search '홍길동전' → 5건");
  });
});

describe('renderPlanForReview', () => {
  it('계획 JSON·코드 검증·조사 기록을 담는다', () => {
    const plan: EditorialPlan = { intent: 'i', protected: [], axes: [], verifications: [], reviewResponses: [] };
    const s = renderPlanForReview(plan, ['축 2개 — 계약 위반'], '[턴0] read 0~100');
    expect(s).toContain('<코드 검증>');
    expect(s).toContain('계약 위반');
    expect(s).toContain('<조사 기록>');
  });
});

describe('renderEditorialComposeReviewInput', () => {
  it('지적 0건 축까지 검토 관점 블록에 담는다', () => {
    const s = renderEditorialComposeReviewInput(
      RESEARCH,
      [
        { label: '시제 전환', inquiry: '전환점이 명확한가', findingCount: 2, discardedCount: 0 },
        { label: '연표 정합', inquiry: '두 층위의 시간이 맞물리는가', findingCount: 0, discardedCount: 0 },
        { label: '공간 정합', inquiry: '동선이 유지되는가', findingCount: 0, discardedCount: 2 },
      ],
      [{ category: '시제 전환', body: '전환점이 흐리다', anchorCount: 1 }],
      [],
    );
    expect(s).toContain('<검토 관점 — 계획이 세우고 전문에 적용한 축>');
    expect(s).toContain('- 시제 전환 · 지적 2건');
    expect(s).toContain('- 연표 정합 · 지적 0건');
    expect(s).toContain('- 공간 정합 · 지적 0건 · 제출 유실 2건(무혐의 판정 불가)');
    expect(s).toContain('검토 질문: 두 층위의 시간이 맞물리는가');
  });
});

describe('renderRejection', () => {
  it('사유 전건을 담는다', () => {
    const s = renderRejection(['앵커 미실재', '축 없음']);
    expect(s).toContain('앵커 미실재');
    expect(s).toContain('축 없음');
  });
});
