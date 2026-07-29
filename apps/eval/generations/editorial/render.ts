// Editorial 단계 입력 렌더러. 워크플로 본문에서 분리해 프롬프트 조정이 쉽도록 한다.
import type { ToolRecord } from './ledger.ts';
import type { AcceptedFinding, AcceptedStrength, EditorialPlan, PlanReviewFinding, ResolvedResearch } from './types.ts';

const COMPLETENESS_LABEL = { draft: '초고', 'in-revision': '퇴고 중', complete: '완성고', undetermined: '판정 불가' } as const;

// 구버전 캐시(문자열 completeness) 리플레이에 방어적이어야 한다.
const renderCompleteness = (c: ResolvedResearch['nature']['completeness']): string =>
  typeof c === 'string' ? c : `${COMPLETENESS_LABEL[c.level]} — ${c.note}`;

// RESEARCH 산출을 규약으로 렌더한다 — PLAN·EXECUTE·COMPOSE의 캐시 접두부.
export const renderResearchCharter = (r: ResolvedResearch): string =>
  [
    '<규약>',
    '[글의 성격]',
    `형식: ${r.nature.form}`,
    `완성도: ${renderCompleteness(r.nature.completeness)}`,
    `유효한 검토의 한계: ${r.nature.feedbackFit}`,
    '',
    '[문체 규약]',
    `시점·화자: ${r.voice.pov}`,
    '의도적 관습 — 작가의 선택이며 결함이 아님:',
    ...r.voice.conventions.map((c) => `  · ${c.pattern}\n    (${c.evidence.join(' / ')})`),
    '',
    '[고유명사]',
    ...r.names.map((n) => `  · ${n.name}${n.aliases.length > 0 ? ` (호칭: ${n.aliases.join(', ')})` : ''}${n.note ? ` — ${n.note}` : ''}`),
    '',
    '[독자 전제]',
    r.premise.sourceWork.status === 'identified'
      ? `원작: ${r.premise.sourceWork.name}\n${r.premise.sourceWork.brief}`
      : '원작: 특정되지 않음(오리지널 취급)',
    r.premise.genreConventions ? `장르 문법:\n${r.premise.genreConventions}` : '',
    r.premise.seriesContext ? `연작 맥락: ${r.premise.seriesContext}` : '',
    '',
    r.boundaries.length > 0 ? `[분석 제외 구간] ${r.boundaries.map((b) => b.reason).join(' / ')}` : '',
    r.unverified.length > 0 ? `[미확인 전제]\n${r.unverified.map((u) => `  · ${u}`).join('\n')}` : '',
    '</규약>',
  ]
    .filter((line) => line !== '')
    .join('\n');

// 계획을 EXECUTE 접두부용으로 렌더한다. reviewResponses·verifications는 검토자에게 불필요하다.
export const renderEditorialPlanBlock = (p: EditorialPlan): string =>
  [
    '<비평 계획>',
    `이 글이 하려는 것: ${p.intent}`,
    '',
    '보호 기법 — 이 글이 효과를 만드는 데 쓰고 있는 것. 지적하지 않는다:',
    ...p.protected.map((x) => `  · ${x.technique} — ${x.rationale}\n    (${x.evidence.join(' / ')})`),
    '',
    '검토 축 — 계획이 정한 살펴볼 곳:',
    ...p.axes.map((a) => `  · ${a.label}: ${a.inquiry}\n    위험: ${a.risk}`),
    '</비평 계획>',
  ].join('\n');

// 원장의 도구 호출 원시 목록 — 검수자·진단이 읽는다.
export const renderToolTrail = (tools: ToolRecord[]): string =>
  tools
    .map((t) => {
      if (t.tool === 'read') return `[턴${t.turn}] read ${t.start}~${t.end}`;
      if (t.tool === 'grep') return `[턴${t.turn}] grep '${t.pattern}' → ${t.total}건`;
      return `[턴${t.turn}] search '${t.query}' → ${t.hits}건`;
    })
    .join('\n');

// 검수자 입력 — 계획 전문 + 코드 검증 결과 + 조사 기록. 직렬화는 압축한다(비용).
export const renderPlanForReview = (p: EditorialPlan, checkNotes: string[], toolTrail: string): string =>
  [
    '<비평 계획>',
    JSON.stringify(p),
    '</비평 계획>',
    '',
    '<코드 검증>',
    checkNotes.length > 0 ? checkNotes.map((n) => `- ${n}`).join('\n') : '위반 없음 — 모든 인용이 원고에서 확인됨',
    '</코드 검증>',
    '',
    '<조사 기록>',
    toolTrail || '(도구 사용 없음)',
    '</조사 기록>',
  ].join('\n');

// 검수 발견을 수정 라운드 입력으로 렌더한다.
export const renderReviewFindingsForRevise = (findings: PlanReviewFinding[]): string =>
  [
    '<검수 발견>',
    ...findings.map(
      (f) =>
        `- [${f.blocking ? '차단' : '권고'}·${f.kind}] ${f.target}\n  문제: ${f.rationale}\n  규약 대조: ${f.conventionsCheck}\n  수정 지시: ${f.fix}`,
    ),
    '</검수 발견>',
    '',
    '검수 발견을 반영해 계획을 다시 제출하세요. 차단 발견은 반드시 처분(반영 또는 근거 있는 기각)하고, 권고는 취사할 수 있습니다. 발견마다 reviewResponses에 처분(adopted/partial/rejected)과 사유를 남기세요 — 원고·규약과 어긋나는 발견은 기각이 정당합니다. 발견이 요구하지 않은 보호·축을 제거하지 마세요.',
  ].join('\n');

// 지적은 호출부가 원고 순서로 정렬해 넘긴다. 좌표는 입력에 넣지 않는다 —
// 작성자가 "1,700자께" 식으로 본문에 반향하는 누출이 실측됐다.
export const renderComposeInputV2 = (findings: AcceptedFinding[]): string =>
  [
    `<지적 ${findings.length}건>`,
    ...findings.map((f, i) =>
      [
        `[${i}] 축: ${f.axis}`,
        `  인용: ${f.quoteStart} … ${f.quoteEnd}`,
        `  의도: ${f.intent}`,
        `  관찰: ${f.observation}`,
        `  원인: ${f.cause}`,
        `  방향: ${f.direction}`,
        `  근거: ${f.evidence}`,
      ].join('\n'),
    ),
    '</지적>',
  ].join('\n');

// 총평 입력 — 규약의 성격·문체가 프로필을 대신한다. 검토 관점 블록은 지적 0건 축까지 담는다 —
// cleared가 계획 문면에 근거를 두게 하는 유일한 통로다.
export const renderEditorialComposeReviewInput = (
  research: ResolvedResearch,
  axes: { label: string; inquiry: string; findingCount: number; discardedCount: number }[],
  feedbacks: { category: string; body: string; anchorCount: number }[],
  strengths: AcceptedStrength[],
): string =>
  [
    '<작품 규약>',
    `형식: ${research.nature.form}`,
    `완성도: ${renderCompleteness(research.nature.completeness)}`,
    `유효한 검토의 한계: ${research.nature.feedbackFit}`,
    `시점: ${research.voice.pov}`,
    `의도적 관습: ${research.voice.conventions.map((c) => c.pattern).join(' / ')}`,
    '</작품 규약>',
    '',
    '<검토 관점 — 계획이 세우고 전문에 적용한 축>',
    ...axes.map(
      (a) =>
        `- ${a.label} · 지적 ${a.findingCount}건${a.discardedCount > 0 ? ` · 제출 유실 ${a.discardedCount}건(무혐의 판정 불가)` : ''}\n  검토 질문: ${a.inquiry}`,
    ),
    '</검토 관점>',
    '',
    '<확정된 피드백>',
    ...feedbacks.map((f, i) => `[${i}] ${f.category} · ${f.anchorCount}곳\n  ${f.body}`),
    '</확정된 피드백>',
    '',
    strengths.length > 0 ? '<강점 후보 — 여기서 고르고 새로 만들지 말 것>' : '',
    ...strengths.map((s) => [`인용: ${s.quoteStart} … ${s.quoteEnd}`, `  ${s.principle}`].join('\n')),
    strengths.length > 0 ? '</강점 후보>' : '',
  ]
    .filter((line) => line !== '')
    .join('\n');

export const renderRejection = (reasons: string[]): string =>
  `반려되었습니다.\n${reasons.map((r) => `- ${r}`).join('\n')}\n고쳐서 다시 제출하세요.`;
