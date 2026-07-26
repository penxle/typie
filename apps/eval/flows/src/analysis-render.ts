import type { Finding, Scene, WorkProfile } from './analysis-types.ts';

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

export const renderReviewInput = (
  profileText: string,
  scenes: Scene[],
  window: { text: string; head: string; tail: string; start: number; end: number },
): string =>
  [
    '<작품 규약>',
    profileText,
    '</작품 규약>',
    '',
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

// 검증에 필요한 만큼만 원문을 준다. 앵커가 놓인 장면과 앞뒤 한 장면씩이면
// "앞에서 이미 나왔다"류의 지적도 확인할 수 있다. 전문을 매번 넣으면 비용이 문서 길이의 제곱으로 는다.
export const relevantText = (content: string, scenes: Scene[], anchors: { matchStart: number | null }[]): string => {
  if (scenes.length === 0) return content;

  const picked = new Set<number>();
  for (const anchor of anchors) {
    const at = anchor.matchStart;
    if (at === null) return content;
    const index = scenes.findIndex((s) => at >= s.start && at < s.end);
    if (index === -1) return content;
    for (const i of [index - 1, index, index + 1]) {
      if (i >= 0 && i < scenes.length) picked.add(i);
    }
  }

  const ordered = [...picked].toSorted((a, b) => a - b);
  const parts: string[] = [];
  let previous = -1;
  for (const i of ordered) {
    if (previous >= 0 && i > previous + 1) parts.push('\n[…중략…]\n');
    if (previous < 0 && scenes[i].start > 0) parts.push('[…앞부분 생략…]\n');
    parts.push(content.slice(scenes[i].start, scenes[i].end));
    previous = i;
  }
  if ((ordered.at(-1) ?? 0) < scenes.length - 1) parts.push('\n[…뒷부분 생략…]');
  return parts.join('');
};

export const renderVerifyInput = (
  profileText: string,
  excerpt: string,
  isFullText: boolean,
  finding: Finding,
  anchors: { quoteStart: string; quoteEnd: string }[],
): string =>
  [
    '<작품 규약>',
    profileText,
    '</작품 규약>',
    '',
    isFullText ? '<원고>' : '<원고 — 이 지적과 관련된 부분>',
    excerpt,
    '</원고>',
    '',
    '<지적>',
    `종류: ${finding.kind}`,
    `의도: ${finding.intent}`,
    `관찰: ${finding.observation}`,
    `원인: ${finding.cause}`,
    `방향: ${finding.direction}`,
    `근거: ${finding.evidence}`,
    '</지적>',
    '',
    '<이 지적이 가리키는 위치>',
    ...anchors.map((a, k) => `[${k}] ${a.quoteStart} … ${a.quoteEnd}`),
    '</이 지적이 가리키는 위치>',
  ].join('\n');

export const renderComposeInput = (
  groups: { representative: number; anchors: { quoteStart: string; quoteEnd: string }[] }[],
  findings: Finding[],
): string =>
  `<검증을 마친 지적>\n${groups
    .map((g, i) => {
      const rep = findings[g.representative];
      return [
        `[${i}] ${rep.kind} · ${g.anchors.length}곳`,
        `  인용: ${g.anchors.map((a) => `${a.quoteStart} … ${a.quoteEnd}`).join(' / ')}`,
        `  의도: ${rep.intent}`,
        `  관찰: ${rep.observation}`,
        `  원인: ${rep.cause}`,
        `  방향: ${rep.direction}`,
      ].join('\n');
    })
    .join('\n\n')}\n</검증을 마친 지적>`;

export const renderComposeReviewInput = (
  profile: ResolvedProfile,
  feedbacks: { category: string; polarity: string; body: string; anchors: unknown[] }[],
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
    ...feedbacks.map((f, i) => `[${i}] ${f.category}(${f.polarity}) · ${f.anchors.length}곳\n  ${f.body}`),
    '</확정된 피드백>',
  ].join('\n');

export const renderBackgroundInput = (query: string, hits: string): string =>
  ['<찾은 것>', `질의: ${query}`, '', hits, '</찾은 것>'].join('\n');
