// cspell:ignore focalization anachronies

// prism high 워크플로 산출물 미러 — 원본: prism apps/feedback/stages/{description,interpretation,rubric,judgment,stylistic}.ts.
// 화면이 읽는 필드만 선언하고 나머지는 통과시킨다(looseObject). enum은 문자열로 받는다 — 값을 그대로 칩으로
// 찍을 뿐 분기하지 않으므로 prism이 값을 늘려도 여기서 깨지지 않는다. 문자열 잎은 결측·null이어도 빈 값으로
// 눕혀 한 잎 때문에 파일 전체가 형식 불일치로 서지 않게 한다. 목록·객체 골격이 다르면 그때가 형식 불일치다.
// prism 쪽 개정 시 이 파일을 함께 갱신한다(types.ts의 미러 관례).

import { parse } from 'yaml';
import { z } from 'zod';
import { lenient } from './lenient-yaml.ts';
import type { ShapeSchema } from './lenient-yaml.ts';
import type { StageKey } from './stages.ts';

export const ARTIFACT_ORDER = [
  'movements',
  'narration',
  'experience',
  'audience',
  'condition',
  'interpretation',
  'rubric',
  'judgment',
  'stylistic',
] as const;

export type ArtifactName = (typeof ARTIFACT_ORDER)[number];

export const ARTIFACT_PATHS: Record<ArtifactName, string> = {
  movements: 'artifacts/movements.yaml',
  narration: 'artifacts/narration.yaml',
  experience: 'artifacts/experience.yaml',
  audience: 'artifacts/audience.yaml',
  condition: 'artifacts/condition.yaml',
  interpretation: 'artifacts/interpretation.yaml',
  rubric: 'artifacts/rubric.yaml',
  judgment: 'artifacts/judgment.yaml',
  stylistic: 'artifacts/stylistic.yaml',
};

// 산출물을 낳은 파이프라인 단계 — 라벨은 stages.ts의 단계 어휘를 그대로 쓴다(과정 화면과 같은 층위·같은 말).
// 레일의 그룹과 섹션의 아이브로가 이 축으로 선다: 산출물 순서가 곧 검토 순서라는 사실을 구조로 보인다.
export const ARTIFACT_STAGES: Record<ArtifactName, StageKey> = {
  movements: 'description',
  narration: 'description',
  experience: 'description',
  audience: 'description',
  condition: 'description',
  interpretation: 'interpretation',
  rubric: 'rubric',
  judgment: 'judgment',
  stylistic: 'stylistic',
};

// prism 계약(Contract.label)의 한국어 이름 그대로 — 번역이 아니라 산출물 자체의 이름이다.
export const ARTIFACT_LABELS: Record<ArtifactName, string> = {
  movements: '구획 지도',
  narration: '서사 체계 카드',
  experience: '독서 기록',
  audience: '상정 독자 카드',
  condition: '원고 상태 카드',
  interpretation: '해석',
  rubric: '기준표',
  judgment: '판정',
  stylistic: '문면 검토',
};

const text = z.string().catch('');
const texts = z.array(z.string()).catch([]);
const quote = { head: text, tail: text };

const MovementsSchema = z.looseObject({
  movements: z.array(z.looseObject({ id: text, ...quote, title: text, basis: text, mode: text, says: text, does: text })),
});

const NarrationSchema = z.looseObject({
  voice: z.looseObject({ type: text, note: text, evidence: texts }),
  situation: text,
  overtness: z.looseObject({ type: text, note: text }),
  focalization: z.looseObject({ type: text, pattern: text, reflectors: texts }),
  tense: z.looseObject({
    base: text,
    anachronies: z.array(z.looseObject({ id: text, kind: text, subjectivity: text, ...quote, note: text })),
  }),
  discourse: z.array(z.looseObject({ id: text, form: text, ...quote, note: text })),
  denomination: z.array(z.looseObject({ id: text, name: text, aliases: texts, note: text })),
  reliability: z.looseObject({ note: text }),
});

const ExperienceSchema = z.looseObject({
  entries: z.array(z.looseObject({ id: text, ...quote, kind: text, note: text })),
});

const AudienceSchema = z.looseObject({
  source: z.looseObject({ status: text, name: text, background: text }),
  genre: text,
  knowledge: z.array(z.looseObject({ id: text, fact: text, source: text, note: text })),
});

const ConditionSchema = z.looseObject({
  completeness: z.looseObject({ level: text, note: text }),
  exclusions: z.array(z.looseObject({ ...quote, reason: text })),
});

const InterpretationSchema = z.looseObject({
  hypothesis: z.looseObject({
    statement: text,
    effect: text,
    questions: z.array(z.looseObject({ id: text, question: text })),
  }),
  performances: z.array(z.looseObject({ id: text, evidence: texts, rationale: text })),
  meanings: z.array(z.looseObject({ id: text, ...quote, principle: text })),
});

const RubricSchema = z.looseObject({
  traits: z.array(
    z.looseObject({
      id: text,
      rationale: text,
      guide: z.looseObject({
        findings: z.array(z.looseObject({ id: text, condition: text })),
        waivers: texts,
        scores: z.array(z.looseObject({ point: z.number(), condition: text })),
        edges: texts,
        verification: text,
      }),
    }),
  ),
  coverage: z.array(
    z.looseObject({
      subject: text,
      from: text,
      disposition: text,
      trait: z.string().nullable().catch(null),
      note: text,
    }),
  ),
});

const verification = z.looseObject({ note: text, method: text });

// 재검토 회차 전용 장부 — 1회차 파일에는 키 자체가 없다(prism followup.ts THREADS_SCHEMA).
const threads = z
  .array(z.looseObject({ thread: text, verdict: text, note: text, anchor: z.looseObject(quote).nullable().optional() }))
  .optional();

const JudgmentSchema = z.looseObject({
  verdicts: z.array(z.looseObject({ trait: text, point: z.number(), note: text, basis: z.string().nullable().optional() })),
  findings: z.array(z.looseObject({ id: text, trait: text, condition: text, ...quote, observation: text, verification, direction: text })),
  // 격상은 3점 특질이 있을 때만 서는 목록이지만 키는 늘 있다 — 이 키가 생기기 전 파일을 위해 결측을 빈 목록으로 눕힌다.
  elevations: z.array(z.looseObject({ id: text, trait: text, ...quote, observation: text, direction: text })).catch([]),
  log: z.array(z.looseObject({ entry: text, disposition: text, finding: z.string().nullable().optional(), note: text })),
  gaps: z.array(z.looseObject({ id: text, note: text })),
  threads,
});

const StylisticSchema = z.looseObject({
  findings: z.array(z.looseObject({ id: text, criterion: text, ...quote, observation: text, verification, direction: text })),
  log: z.array(z.looseObject({ entry: text, disposition: text, finding: z.string().nullable().optional(), note: text })),
  coverage: z.array(z.looseObject({ movement: text, note: text })),
  threads,
});

const SCHEMAS = {
  movements: MovementsSchema,
  narration: NarrationSchema,
  experience: ExperienceSchema,
  audience: AudienceSchema,
  condition: ConditionSchema,
  interpretation: InterpretationSchema,
  rubric: RubricSchema,
  judgment: JudgmentSchema,
  stylistic: StylisticSchema,
} satisfies Record<ArtifactName, z.ZodType>;

export type Movements = z.infer<typeof MovementsSchema>;
export type Narration = z.infer<typeof NarrationSchema>;
export type Experience = z.infer<typeof ExperienceSchema>;
export type Audience = z.infer<typeof AudienceSchema>;
export type Condition = z.infer<typeof ConditionSchema>;
export type Interpretation = z.infer<typeof InterpretationSchema>;
export type Rubric = z.infer<typeof RubricSchema>;
export type Judgment = z.infer<typeof JudgmentSchema>;
export type Stylistic = z.infer<typeof StylisticSchema>;

export type ArtifactValue = { [K in ArtifactName]: z.infer<(typeof SCHEMAS)[K]> };

export type ParsedArtifact<T> = { status: 'ok'; value: T } | { status: 'missing' } | { status: 'invalid' };

export type Artifacts = { [K in ArtifactName]: ParsedArtifact<ArtifactValue[K]> };

// 관용 전처리용 형상 — prism JSON 스키마의 type 골격 미러(전 필드; 화면이 안 읽는 필드도 prism이 string으로 선언했으면 감싼다).
// 원본: prism stages/{description,interpretation,rubric,judgment,stylistic}.ts, followup.ts THREADS_SCHEMA.
const str: ShapeSchema = { type: 'string' };
const int: ShapeSchema = { type: 'integer' };
const strList: ShapeSchema = { type: 'array', items: str };
const obj = (properties: Record<string, ShapeSchema>): ShapeSchema => ({ type: 'object', properties });
const arr = (items: ShapeSchema): ShapeSchema => ({ type: 'array', items });
const quoteShape = { head: str, tail: str };
const verificationShape = obj({ note: str, method: str });
const threadsShape = arr(obj({ thread: str, verdict: str, note: str, anchor: obj(quoteShape) }));

const SHAPES: Record<ArtifactName, ShapeSchema> = {
  movements: obj({ movements: arr(obj({ id: str, ...quoteShape, title: str, basis: str, mode: str, says: str, does: str })) }),
  narration: obj({
    voice: obj({ type: str, note: str, evidence: strList }),
    situation: str,
    overtness: obj({ type: str, note: str }),
    focalization: obj({ type: str, pattern: str, reflectors: strList }),
    tense: obj({ base: str, anachronies: arr(obj({ id: str, kind: str, subjectivity: str, ...quoteShape, note: str })) }),
    discourse: arr(obj({ id: str, form: str, ...quoteShape, note: str })),
    denomination: arr(obj({ id: str, name: str, aliases: strList, note: str })),
    reliability: obj({ note: str }),
  }),
  experience: obj({ entries: arr(obj({ id: str, ...quoteShape, kind: str, note: str })) }),
  audience: obj({
    source: obj({ status: str, name: str, background: str }),
    genre: str,
    knowledge: arr(obj({ id: str, fact: str, source: str, note: str })),
  }),
  condition: obj({ completeness: obj({ level: str, note: str }), exclusions: arr(obj({ ...quoteShape, reason: str })) }),
  interpretation: obj({
    hypothesis: obj({ statement: str, effect: str, questions: arr(obj({ id: str, question: str })) }),
    performances: arr(obj({ id: str, evidence: strList, rationale: str })),
    meanings: arr(obj({ id: str, ...quoteShape, principle: str })),
  }),
  rubric: obj({
    traits: arr(
      obj({
        id: str,
        rationale: str,
        guide: obj({
          findings: arr(obj({ id: str, condition: str })),
          waivers: strList,
          scores: arr(obj({ point: int, condition: str })),
          edges: strList,
          verification: str,
        }),
      }),
    ),
    // trait은 prism 스키마에 type이 없다(covered면 id, dismissed면 null) — 표준 규칙 그대로.
    coverage: arr(obj({ subject: str, from: str, disposition: str, trait: {}, note: str })),
  }),
  judgment: obj({
    verdicts: arr(obj({ trait: str, point: int, note: str, basis: str })),
    findings: arr(
      obj({ id: str, trait: str, condition: str, ...quoteShape, observation: str, verification: verificationShape, direction: str }),
    ),
    elevations: arr(obj({ id: str, trait: str, ...quoteShape, observation: str, direction: str })),
    log: arr(obj({ entry: str, disposition: str, finding: str, note: str })),
    gaps: arr(obj({ id: str, note: str })),
    threads: threadsShape,
  }),
  stylistic: obj({
    findings: arr(obj({ id: str, criterion: str, ...quoteShape, observation: str, verification: verificationShape, direction: str })),
    log: arr(obj({ entry: str, disposition: str, finding: str, note: str })),
    coverage: arr(obj({ movement: str, note: str })),
    threads: threadsShape,
  }),
};

export const parseArtifact = <K extends ArtifactName>(name: K, raw: string | null): ParsedArtifact<ArtifactValue[K]> => {
  if (raw === null) return { status: 'missing' };
  let data: unknown;
  try {
    // prism과 같은 관용 전처리를 거쳐 읽는다 — prism이 접수한 파일은 여기서도 읽혀야 한다.
    data = parse(lenient(raw, SHAPES[name]));
  } catch {
    return { status: 'invalid' };
  }
  const result = SCHEMAS[name].safeParse(data);
  return result.success ? { status: 'ok', value: result.data as ArtifactValue[K] } : { status: 'invalid' };
};

// 레일 배지 건수 — 목록형 산출물은 주 목록의 길이, 카드형(서사 체계·원고 상태)은 섹션 전체가 카드 하나라 1(오너 지시).
// 산출물이 서 있지 않으면(missing·invalid) 배지 없음.
export const countOf = (name: ArtifactName, artifacts: Artifacts): number | null => {
  switch (name) {
    case 'movements': {
      const p = artifacts.movements;
      return p.status === 'ok' ? p.value.movements.length : null;
    }
    case 'experience': {
      const p = artifacts.experience;
      return p.status === 'ok' ? p.value.entries.length : null;
    }
    case 'audience': {
      const p = artifacts.audience;
      return p.status === 'ok' ? p.value.knowledge.length : null;
    }
    case 'interpretation': {
      const p = artifacts.interpretation;
      return p.status === 'ok' ? p.value.performances.length : null;
    }
    case 'rubric': {
      const p = artifacts.rubric;
      return p.status === 'ok' ? p.value.traits.length : null;
    }
    case 'judgment': {
      const p = artifacts.judgment;
      return p.status === 'ok' ? p.value.findings.length : null;
    }
    case 'stylistic': {
      const p = artifacts.stylistic;
      return p.status === 'ok' ? p.value.findings.length : null;
    }
    default: {
      return artifacts[name].status === 'ok' ? 1 : null;
    }
  }
};

// 모달 안 교차 참조의 앵커 id — 산출물 사이의 id 참조를 같은 화면 안의 자리로 잇는다(원고로 가는 링크는 없다).
export const anchorOf = {
  movement: (id: string) => `af-movement-${id}`,
  experience: (id: string) => `af-experience-${id}`,
  performance: (id: string) => `af-performance-${id}`,
  question: (id: string) => `af-question-${id}`,
  trait: (id: string) => `af-rubric-trait-${id}`,
  condition: (trait: string, condition: string) => `af-rubric-cond-${trait}-${condition}`,
  judgmentFinding: (id: string) => `af-judgment-finding-${id}`,
  stylisticFinding: (id: string) => `af-stylistic-finding-${id}`,
};

// 실재하는 앵커의 집합 — 참조 칩은 대상이 이 집합에 있을 때만 링크가 된다.
export const refTargets = (artifacts: Artifacts): Set<string> => {
  const targets = new Set<string>();
  const add = (id: string, make: (id: string) => string) => {
    if (id) targets.add(make(id));
  };
  if (artifacts.movements.status === 'ok') for (const m of artifacts.movements.value.movements) add(m.id, anchorOf.movement);
  if (artifacts.experience.status === 'ok') for (const e of artifacts.experience.value.entries) add(e.id, anchorOf.experience);
  if (artifacts.interpretation.status === 'ok') {
    for (const p of artifacts.interpretation.value.performances) add(p.id, anchorOf.performance);
    for (const q of artifacts.interpretation.value.hypothesis.questions) add(q.id, anchorOf.question);
  }
  if (artifacts.rubric.status === 'ok') {
    for (const trait of artifacts.rubric.value.traits) {
      add(trait.id, anchorOf.trait);
      for (const finding of trait.guide.findings) {
        if (trait.id && finding.id) targets.add(anchorOf.condition(trait.id, finding.id));
      }
    }
  }
  if (artifacts.judgment.status === 'ok') for (const f of artifacts.judgment.value.findings) add(f.id, anchorOf.judgmentFinding);
  if (artifacts.stylistic.status === 'ok') for (const f of artifacts.stylistic.value.findings) add(f.id, anchorOf.stylisticFinding);
  return targets;
};

// 대상이 실재할 때만 앵커를 돌려준다 — 칩은 이 값이 있으면 링크, 없으면 정적 표시다.
export const linkTo = (targets: Set<string>, anchor: string): string | undefined => (targets.has(anchor) ? anchor : undefined);
