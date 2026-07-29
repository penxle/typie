import type { Finding, Plan, Scene, WorkProfile } from './analysis-types.ts';

// 단계별 사용자 메시지 구성. 워크플로 본문에서 분리해 두어 프롬프트 조정이 쉽도록 한다.

export type ResolvedProfile = WorkProfile & { derivativeSource?: string };

// background는 2차 창작의 원작 배경이다. 원고에는 없지만 독자에게는 전제된 지식이라, 이것이
// 없으면 원작 지식에 기댄 생략을 결핍으로 읽는다(라운드 3에서 사유 8건이 이 유형이었다).
// 원고가 아니라 원작을 조사해 얻은 것이므로 본문 근거로는 쓸 수 없다 — 판단을 눌러주는 용도다.
export const renderProfile = (profile: ResolvedProfile, background?: string | null): string =>
  [
    `형식: ${profile.form}`,
    `2차 창작: ${profile.isDerivative ? `예 — ${profile.derivativeSource ?? '원작 불명'}` : '아니오'}`,
    background ? `\n[원작 배경 — 조사한 참고 정보]\n${background}\n` : '',
    `시점: ${profile.pov}`,
    `화자 신뢰성: ${profile.reliability}`,
    `시제: ${profile.tense}`,
    `대사 표기: ${profile.dialogueConvention}`,
    '',
    '의도적 문체 — 작가의 선택이며 결함이 아님:',
    ...profile.deliberateStyles.map((s) => `  · ${s.pattern}\n    (${s.evidence})`),
    '',
    `고유명사·설정 용어: ${profile.properNouns.join(', ')}`,
    profile.nonAnalyticRanges.length > 0 ? `분석 대상 아닌 구간: ${profile.nonAnalyticRanges.map((r) => r.reason).join(' / ')}` : '',
  ]
    .filter(Boolean)
    .join('\n');

// 작품 규약은 한 문서의 모든 호출이 똑같이 다시 보내는 부분이다. 프롬프트 캐싱이 걸리도록
// 앞머리로 떼어 둔다 — 캐시 대상은 프롬프트의 접두부여야 한다.
export const renderConventions = (profileText: string): string => ['<작품 규약>', profileText, '</작품 규약>'].join('\n');

// 계획도 규약처럼 한 문서의 모든 검토 호출이 공유한다 — 같은 접두부 층에 싣는다.
export const renderPlan = (plan: Plan): string =>
  [
    '<비평 계획>',
    `이 글이 하려는 것: ${plan.intent}`,
    '',
    '보호 기법 — 이 글이 효과를 만드는 데 쓰고 있는 것. 지적하지 않는다:',
    ...plan.protected.map((p) => `  · ${p.technique} — ${p.rationale}\n    (${p.evidence.join(' / ')})`),
    '',
    '검토 축 — 살펴볼 곳은 이것이 전부다:',
    ...plan.axes.map((a) => `  · ${a.label}: ${a.description}\n    위험: ${a.risk}`),
    '</비평 계획>',
  ].join('\n');

// 검수자에게는 계획과 함께 코드 검증 결과를 준다 — 인용 미실재 같은 기계적 사실이
// 검수의 출발점이 되도록.
export const renderPlanReviewInput = (plan: Plan, checkNotes: string[]): string =>
  [
    '<비평 계획>',
    JSON.stringify(plan, null, 2),
    '</비평 계획>',
    '',
    '<코드 검증>',
    checkNotes.length > 0 ? checkNotes.map((n) => `- ${n}`).join('\n') : '위반 없음 — 모든 인용이 원고에서 확인됨',
    '</코드 검증>',
  ].join('\n');

// 검수 발견을 받아 계획을 고친다. 원 계획과 발견을 함께 주고 같은 도구로 다시 받는다.
export const renderPlanReviseInput = (
  plan: Plan,
  findings: { kind: string; target: string; rationale: string; fix: string; genreCheck: string; confidence: number }[],
): string =>
  [
    '<원래 계획>',
    JSON.stringify(plan, null, 2),
    '</원래 계획>',
    '',
    '<검수 발견>',
    ...findings.map(
      (f) =>
        `- [${f.kind} · 확신 ${f.confidence}] ${f.target}\n  문제: ${f.rationale}\n  장르 대조: ${f.genreCheck}\n  수정 지시: ${f.fix}`,
    ),
    '</검수 발견>',
    '',
    '검수 발견을 반영해 계획을 다시 세우세요. 타당한 발견만 반영하고, 원고·원작·장르 관습과 어긋나는 발견은 기각하되 rejectedFindings에 대상과 사유를 남기세요. 발견이 요구하지 않은 보호·축을 제거하지 마세요.',
  ].join('\n');

export const renderReviewInput = (
  scenes: Scene[],
  window: { text: string; head: string; tail: string; start: number; end: number },
): string =>
  [
    '<이 구간의 장면>',
    scenes
      .filter((s) => s.start >= window.start && s.end <= window.end)
      .map((s) => `· ${s.gist} (${s.setting}, ${s.pov})${s.flashback ? ` [회상: ${s.flashback}]` : ''}`)
      .join('\n'),
    '</이 구간의 장면>',
    '',
    window.head ? `[앞 문맥 — 분석 대상 아님]\n${window.head}\n` : '',
    '[분석 대상]',
    window.text,
    '[분석 대상 끝]',
    window.tail ? `\n[뒤 문맥 — 분석 대상 아님]\n${window.tail}` : '',
  ]
    .filter(Boolean)
    .join('\n');

export const renderDedupeInput = (findings: Finding[]): string =>
  `<지적 목록>\n${findings
    .map((f, i) =>
      [
        `[${i}] ${f.kind} · 위치 ${f.matchStart ?? '?'}`,
        `  인용: ${f.quoteStart} … ${f.quoteEnd}`,
        `  의도: ${f.intent}`,
        `  관찰: ${f.observation}`,
        `  원인: ${f.cause}`,
      ].join('\n'),
    )
    .join('\n\n')}\n</지적 목록>`;

// 검증에 필요한 만큼만 원문을 준다. 전문을 매번 넣으면 비용이 문서 길이의 제곱으로 는다.
// 어느 장면이 필요한지는 verify-batch가 정한다 — 호출 편성과 발췌가 같은 기준을 써야
// 한 호출에 묶인 지적들이 실제로 같은 원문을 읽는다.
export const renderScenes = (content: string, scenes: Scene[], sceneIndexes: number[] | null): string => {
  if (sceneIndexes === null || sceneIndexes.length === 0) return content;

  const parts: string[] = [];
  let previous = -1;
  for (const i of sceneIndexes) {
    if (previous >= 0 && i > previous + 1) parts.push('\n[…중략…]\n');
    if (previous < 0 && scenes[i].start > 0) parts.push('[…앞부분 생략…]\n');
    parts.push(content.slice(scenes[i].start, scenes[i].end));
    previous = i;
  }
  if ((sceneIndexes.at(-1) ?? 0) < scenes.length - 1) parts.push('\n[…뒷부분 생략…]');
  return parts.join('');
};

// 같은 원문을 공유하는 지적들을 한 번에 싣는다. 원문을 지적 수만큼 다시 보내는 것이
// 이 파이프라인 입력 비용의 대부분이었다(실측 68~87%).
//
// 판정은 여전히 지적마다 독립이다. 한 호출에 담기는 것은 읽을 원문이 같기 때문이지
// 서로 견주라는 뜻이 아니며, 그 사실을 입력에도 못박아 둔다.
export const renderVerifyInput = (
  excerpt: string,
  isFullText: boolean,
  items: { finding: Finding; anchors: { quoteStart: string; quoteEnd: string }[] }[],
): string =>
  [
    isFullText ? '<원고>' : '<원고 — 아래 지적들과 관련된 부분>',
    excerpt,
    '</원고>',
    '',
    `<판정할 지적 ${items.length}건 — 서로 견주지 말고 하나씩 판정할 것>`,
    ...items.flatMap(({ finding, anchors }, i) => [
      `[지적 ${i}]`,
      `  종류: ${finding.kind}`,
      `  의도: ${finding.intent}`,
      `  관찰: ${finding.observation}`,
      `  원인: ${finding.cause}`,
      `  방향: ${finding.direction}`,
      `  근거: ${finding.evidence}`,
      '  가리키는 위치:',
      ...anchors.map((a, k) => `    [${k}] ${a.quoteStart} … ${a.quoteEnd}`),
      '',
    ]),
    '</판정할 지적>',
  ].join('\n');

export const renderComposeInput = (
  groups: { representative: number; anchors: { quoteStart: string; quoteEnd: string }[] }[],
  findings: Finding[],
): string =>
  [
    `<지적 ${groups.length}건>`,
    groups
      .map((g, i) => {
        const rep = findings[g.representative];
        return [
          `[${i}] ${rep.kind} · ${g.anchors.length}곳`,
          `  인용: ${g.anchors.map((a) => `${a.quoteStart} … ${a.quoteEnd}`).join(' / ')}`,
          `  의도: ${rep.intent}`,
          `  관찰: ${rep.observation}`,
          `  원인: ${rep.cause}`,
          `  방향: ${rep.direction}`,
          `  근거: ${rep.evidence}`,
        ].join('\n');
      })
      .join('\n\n'),
    '</지적>',
  ]
    .filter(Boolean)
    .join('\n');

// 강점 후보를 여기서 처음 만난다. 짚을 곳과 달리 중복 묶기·피드백 쓰기를 거치지 않으므로
// 다듬어지지 않은 원형 그대로이며, 무엇을 고를지는 총평이 정한다.
export const renderComposeReviewInput = (
  profile: ResolvedProfile,
  feedbacks: { category: string; body: string; anchors: unknown[] }[],
  strengths: { quoteStart: string; quoteEnd: string; principle: string }[],
): string =>
  [
    '<작품 규약>',
    `형식: ${profile.form}`,
    `시점: ${profile.pov}`,
    `시제: ${profile.tense}`,
    `의도적 문체: ${profile.deliberateStyles.map((s) => s.pattern).join(' / ')}`,
    '</작품 규약>',
    '',
    '<확정된 피드백>',
    ...feedbacks.map((f, i) => `[${i}] ${f.category} · ${f.anchors.length}곳\n  ${f.body}`),
    '</확정된 피드백>',
    '',
    strengths.length > 0 ? '<강점 후보 — 여기서 고르고 새로 만들지 말 것>' : '',
    ...strengths.map((s) => [`인용: ${s.quoteStart} … ${s.quoteEnd}`, `  ${s.principle}`].join('\n')),
    strengths.length > 0 ? '</강점 후보>' : '',
  ]
    .filter(Boolean)
    .join('\n');

export const renderBackgroundInput = (query: string, hits: string): string =>
  ['<찾은 것>', `질의: ${query}`, '', hits, '</찾은 것>'].join('\n');

// 자체 검증의 입력. 원고는 여기 넣지 않는다 — 문서의 모든 검증 호출이 공유하므로 캐시
// 접두부(system)로 올린다. 검증 단계가 비쌌던 이유가 발췌를 호출마다 새로 보낸 것이었다.
export const renderSelfCheckInput = (
  feedback: { category: string; body: string },
  anchors: { quoteStart: string; quoteEnd: string }[],
): string =>
  [
    '<판정할 피드백>',
    `분류: ${feedback.category}`,
    feedback.body,
    '</판정할 피드백>',
    '',
    '<이 피드백이 가리키는 위치>',
    ...anchors.map((a, i) => `[${i}] ${a.quoteStart} … ${a.quoteEnd}`),
    '</이 피드백이 가리키는 위치>',
  ].join('\n');
