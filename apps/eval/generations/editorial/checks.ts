// cspell:ignore antml
// Editorial 파이프라인의 코드 검증 — 스펙 §5~§7. 산문 계약은 지켜지지 않으므로
// 인용 실재·열람 범위·축 수·커버리지·대조 참조를 전부 코드가 대조한다.
import { createFindRange } from '../../core/text.ts';
import { grepBefore, readRanges, uncovered, withinRead } from './ledger.ts';
import type { ToolRecord } from './ledger.ts';
import type { AcceptedFinding, EditorialFinding, EditorialPlan, Range, Research, ResolvedResearch } from './types.ts';

// 축 개수의 퇴화 방지선 — 적정 개수의 주장이 아니다. 개수는 문서의 위험 프로파일이 정하고
// 품질은 검수가 공격한다. 코드는 명백한 고장(빈 계획, 나열 폭주)만 반려한다.
export const AXIS_MIN = 1;
export const AXIS_MAX = 12;
export const PROTECTED_MAX = 8;
// 같은 지적의 반려 상한 — 초과하면 폐기하고 기록한다.
export const FILE_REJECT_MAX = 3;
// 단계당 턴 백스톱. 상한이 아니라 폭주 방지선이다.
export const TURN_CAP = 40;

// 도구 호출 XML 구문이 문자열 필드로 새는 직렬화 사고의 검출. 시그니처 열거는 세 번째
// 형태(스키마 필드명 태그 — v1-007 doc1 intent)에 뚫렸다 — ASCII XML 태그 패턴 전체를 본다.
// 한국어 산문·인용에는 <ascii…> 꼴이 나타나지 않는다는 것이 전제다.
const TAG_PATTERN = /<\/?[a-zA-Z][a-zA-Z0-9_]*(?=[\s/>])/;
export const hasToolSyntaxLeak = (values: string[]): boolean => values.some((s) => TAG_PATTERN.test(s) || s.includes('antml'));

// 초안의 줄 단위 오염 스캔 — 위치를 짚어야 국소 제거가 된다. 오염이 초안 파일의 가시적
// 텍스트로 남는 것이 초안 흐름의 계약이고, 이 노트가 그 제거를 안내한다.
export const leakLineLint = (content: string): string[] => {
  const notes: string[] = [];
  for (const [i, line] of content.split('\n').entries()) {
    if (hasToolSyntaxLeak([line])) {
      notes.push(
        `${i + 1}행: 도구 호출 태그(<영문단어> 꼴)가 섞였습니다 — 그 부분만 지우세요. 원고의 특수문자·괄호·기호는 원인이 아니니 내용을 깎지 마세요`,
      );
    }
  }
  return notes;
};

// 도구 출력의 문자 좌표가 서술에 새는 것을 잡는다(실측: evidence의 "1851 '인용', 2183 …" 나열이
// compose를 거쳐 작가 문면에 도달). 인용부 안의 숫자(작품 자체의 수치)는 제외하고, 인용부 밖
// 4~5자리 수가 2개 이상이면 좌표 나열로 본다 — 연도 하나("2020")나 3자리 수치는 통과한다.
const QUOTED_SPAN = /["'“”‘’][^"'“”‘’]*["'“”‘’]/g;
export const hasBareCoordinateList = (values: string[]): boolean =>
  values.some((s) => (s.replaceAll(QUOTED_SPAN, '').match(/(?<![0-9])[0-9]{4,5}(?![0-9])/g) ?? []).length >= 2);

export type QuoteCheck = { range: Range | null; reason: string | null };

// 인용이 (a) 원고에 실재하고 (b) 열람한 범위 안인지. file은 대상 원고 파일 경로다.
export const checkQuote = (content: string, tools: ToolRecord[], quote: string, file: string): QuoteCheck => {
  const findRange = createFindRange(content);
  const found = findRange(quote, quote, 0);
  if (!found) return { range: null, reason: `원고에 없음: ${quote.slice(0, 30)}` };
  if (!withinRead(tools, found.rangeStart, found.rangeEnd, file)) {
    return { range: null, reason: `열람 범위 밖: ${quote.slice(0, 30)}` };
  }
  return { range: { start: found.rangeStart, end: found.rangeEnd }, reason: null };
};

const keepVerifiedQuotes = (
  content: string,
  tools: ToolRecord[],
  quotes: string[],
  where: string,
  notes: string[],
  file: string,
): string[] => {
  const kept: string[] = [];
  for (const q of quotes) {
    const check = checkQuote(content, tools, q, file);
    if (check.range) kept.push(q);
    else notes.push(`${where} — ${check.reason}`);
  }
  return kept;
};

export const checkResearch = (
  content: string,
  research: Research,
  tools: ToolRecord[],
  file: string,
): { research: ResolvedResearch; notes: string[] } => {
  const notes: string[] = [];
  const findRange = createFindRange(content);

  const conventions = [];
  for (const c of research.voice.conventions) {
    const kept = keepVerifiedQuotes(content, tools, c.evidence, `문체 관습 "${c.pattern}"`, notes, file);
    if (kept.length === 0) {
      notes.push(`문체 관습 "${c.pattern}" — 실재하는 근거가 없어 등재 취소`);
      continue;
    }
    conventions.push({ ...c, evidence: kept });
  }

  const boundaryRanges: Range[] = [];
  const boundaries = [];
  let cursor = 0;
  for (const b of research.boundaries) {
    const range = findRange(b.startQuote, b.endQuote, cursor);
    if (!range) {
      notes.push(`제외 구간 "${b.reason}" — 인용을 원고에서 찾지 못해 제거`);
      continue;
    }
    cursor = range.rangeEnd;
    boundaries.push(b);
    boundaryRanges.push({ start: range.rangeStart, end: range.rangeEnd });
  }

  // 검색 없이 원작을 특정했다고 주장하면 미판정으로 강등한다 — 판정 권위는 검색이다.
  let sourceWork = research.premise.sourceWork;
  if (sourceWork.status === 'identified' && tools.every((t) => t.tool !== 'search')) {
    notes.push('원작 특정 주장 — 검색 기록이 없어 미판정으로 강등');
    sourceWork = { ...sourceWork, status: 'undetermined' };
  }

  return {
    research: {
      ...research,
      voice: { ...research.voice, conventions },
      premise: { ...research.premise, sourceWork },
      boundaries,
      boundaryRanges,
    },
    notes,
  };
};

export const checkEditorialPlan = (
  content: string,
  plan: EditorialPlan,
  tools: ToolRecord[],
  file: string,
): { plan: EditorialPlan; contractOk: boolean; notes: string[] } => {
  const notes: string[] = [];

  const protectedKept: EditorialPlan['protected'] = [];
  for (const p of plan.protected) {
    const kept = keepVerifiedQuotes(content, tools, p.evidence, `보호 "${p.technique}"`, notes, file);
    if (kept.length === 0) {
      notes.push(`보호 "${p.technique}" — 실재하는 근거가 없어 등재 취소`);
      continue;
    }
    protectedKept.push({ ...p, evidence: kept });
  }
  if (protectedKept.length > PROTECTED_MAX) {
    notes.push(`보호 ${protectedKept.length}개 — 상한(${PROTECTED_MAX}) 초과, 뒤에서부터 잘라냄`);
    protectedKept.length = PROTECTED_MAX;
  }

  // 축은 근거가 비어도 제거하지 않는다 — 축 수 계약이 조용히 깨지면 안 되고,
  // 근거 없는 축의 처분(weak-axis)은 검수의 판단이다.
  // 검색 수행의 자기 신고는 원장으로 반증한다 — 거짓 신고는 계약 위반으로 반려된다.
  let searchClaimOk = true;
  if (hasToolSyntaxLeak([JSON.stringify(plan)])) {
    notes.push('계획 필드에 도구 호출 구문이 섞임 — 직렬화 사고. 각 필드를 순수 텍스트로 다시 제출하라');
    searchClaimOk = false;
  }
  // 축의 겨냥 필드는 실질 서술이어야 한다 — 스키마는 존재만 보므로 공백·한 단어 채우기는
  // 여기서 잡는다. 라운드 4 실측: 확인 절차형 축(독자 비용 없음)이 지적을 무혐의로 삼켰다.
  let axisAimOk = true;
  const axes = plan.axes.map((a) => {
    const kept = keepVerifiedQuotes(content, tools, a.evidence, `축 "${a.label}"`, notes, file);
    if (kept.length === 0) notes.push(`축 "${a.label}" — 실재하는 근거가 없음`);
    // ?? ''는 구계약 캐시 리플레이 방어 — 옛 plan 값에는 이 필드가 없다.
    if ((a.readerCost ?? '').trim().length < 10) {
      notes.push(`축 "${a.label}" — readerCost가 비어 있거나 형식적임. 이 위험이 실현되면 독자가 무엇을 치르는지 서술하라`);
      axisAimOk = false;
    }
    if ((a.expectedFinding ?? '').trim().length < 10) {
      notes.push(`축 "${a.label}" — expectedFinding이 비어 있거나 형식적임. 이 축이 낳을 지적의 형태를 서술하라`);
      axisAimOk = false;
    }
    if (a.conventionsBasis === 'search' && tools.every((t) => t.tool !== 'search')) {
      notes.push(`축 "${a.label}" — 검색으로 확정했다는데 원장에 search가 없음`);
      searchClaimOk = false;
    }
    return { ...a, evidence: kept };
  });

  // 구버전 캐시 리플레이에서 tools가 없을 수 있다 — 방어적으로 건너뛴다.
  for (const v of plan.verifications) {
    for (const used of v.tools ?? []) {
      if (tools.some((t) => t.tool === used)) {
        continue;
      }

      notes.push(`확정 기록 "${v.question.slice(0, 30)}" — ${used}를 썼다는데 원장에 없음`);
      searchClaimOk = false;
    }
  }

  const axisCountOk = axes.length >= AXIS_MIN && axes.length <= AXIS_MAX;
  if (!axisCountOk) notes.push(`축 ${axes.length}개 — 계약(${AXIS_MIN}~${AXIS_MAX}) 위반`);

  return { plan: { ...plan, protected: protectedKept, axes }, contractOk: axisCountOk && searchClaimOk && axisAimOk, notes };
};

// 확정 기록은 라운드를 넘어 보존된다 — 계획자가 재제출에서 빼먹어도 원장은 줄지 않는다
// (실측: 검색 확정이 수정 라운드에서 통째로 증발). question 동일 항목은 최신 제출본이 이긴다.
export const mergeVerifications = (
  prev: EditorialPlan['verifications'],
  next: EditorialPlan['verifications'],
): EditorialPlan['verifications'] => {
  const questions = new Set(next.map((v) => v.question));
  return [...prev.filter((v) => !questions.has(v.question)), ...next];
};

const startsInAnyRange = (span: Range, ranges: Range[]): boolean => ranges.some((r) => span.start >= r.start && span.start < r.end);

export const checkFinding = (
  content: string,
  finding: EditorialFinding,
  tools: ToolRecord[],
  turn: number,
  axes: string[],
  boundaryRanges: Range[],
  file: string,
): { accepted: AcceptedFinding | null; reasons: string[] } => {
  const reasons: string[] = [];

  if (hasToolSyntaxLeak([JSON.stringify(finding)])) {
    reasons.push('필드에 도구 호출 구문이 섞임 — 각 필드를 순수한 본문으로 다시 제출하라');
  }
  if (hasBareCoordinateList([finding.intent, finding.observation, finding.cause, finding.direction, finding.evidence, finding.stake])) {
    reasons.push('문자 좌표로 위치를 나열함 — 좌표가 아니라 원고 문구 인용으로 가리켜라');
  }
  if (!axes.includes(finding.axis)) reasons.push(`축 "${finding.axis}"는 계획에 없음`);

  const findRange = createFindRange(content);
  const range = findRange(finding.quoteStart, finding.quoteEnd, 0);
  if (range) {
    if (!withinRead(tools, range.rangeStart, range.rangeEnd, file)) reasons.push('앵커가 열람 범위 밖 — 읽은 곳만 지적할 수 있다');
    if (startsInAnyRange({ start: range.rangeStart, end: range.rangeEnd }, boundaryRanges)) {
      reasons.push('분석 제외 구간에 대한 지적');
    }
  } else {
    reasons.push(`앵커를 원고에서 찾지 못함: ${finding.quoteStart.slice(0, 30)}`);
  }

  if (finding.manuscriptBasis === 'grep' && !grepBefore(tools, turn + 1, file)) {
    reasons.push('전문 대조를 했다는데 원장에 grep 기록이 없음');
  }

  if (!range || reasons.length > 0) return { accepted: null, reasons };
  return { accepted: { ...finding, matchStart: range.rangeStart, matchEnd: range.rangeEnd, filedAtTurn: turn }, reasons: [] };
};

export const coverageGaps = (contentLength: number, tools: ToolRecord[], boundaryRanges: Range[], file: string): Range[] =>
  uncovered(contentLength, readRanges(tools, file), boundaryRanges);
