// 산출물 계약 — 스테이지가 무엇을 만들어야 하는지의 단일 정의. 경로→스키마 맵이 유일한
// 진실 원천이고, 검증 파이프라인·모델 안내(가이드)·제출 도구·확정 헤더가 전부 여기서
// 파생된다.
//
// 산출물 포맷은 YAML이다: 산문·인용 필드를 블록 스칼라로 이스케이프 없이 담고(JSON은 따옴표
// 이스케이프·트레일링 콤마가 편집마다 발목을 잡는다), 표준 파서가 줄 위치 있는 오류를 주며,
// 파싱 후에는 input_schema형 검증기를 그대로 재사용한다. 자체 마크다운 문법은 파서와
// 모호성을 새로 소유하게 되어 기각했다.
import { parseDocument } from 'yaml';
import { schemaViolations } from '../tool-schema.ts';
import { fileTools } from './workspace.ts';
import type Anthropic from '@anthropic-ai/sdk';

// 문면 검사 훅 — 줄 위치를 짚는 결정적 검사(예: 직렬화 오염 스캔)를 세대가 꽂는다.
export type Lint = (content: string) => string[];

// 산출물 파일 하나의 계약. description은 색인(후속 스테이지의 발견)과 가이드 절 제목에 쓰인다.
export type OutputSpec = { schema: unknown; lints?: Lint[]; description: string };

export type Deliverable = {
  label: string;
  submitName: string;
  submitDescription: string;
  // 경로 → 계약. output/에는 여기 선언된 경로만 쓸 수 있다(워크스페이스가 강제).
  outputs: Record<string, OutputSpec>;
};

export type Validation<T> = { value?: T; notes: string[] };

const declaredPaths = (deliverable: Deliverable): string => Object.keys(deliverable.outputs).join(', ');

// 저장 시와 제출 시가 같은 검사를 공유한다 — 통과 조건이 둘로 갈리면 "저장은 됐는데 제출이
// 안 되는" 어긋남이 생긴다. 문맥이 필요한 도메인 검사만 제출 처리자(onSubmit)의 몫이다.
export const validateOutput = <T>(deliverable: Deliverable, path: string, content: string | null): Validation<T> => {
  const spec = deliverable.outputs[path];
  if (!spec) return { notes: [`선언되지 않은 산출물 경로입니다: ${path}. 선언된 경로: ${declaredPaths(deliverable)}`] };
  if (content === null) return { notes: [`${path}가 아직 없습니다 — write로 먼저 작성하세요`] };
  const notes: string[] = [];
  for (const lint of spec.lints ?? []) notes.push(...lint(content));
  const doc = parseDocument(content);
  for (const err of doc.errors) {
    const line = err.linePos?.[0]?.line;
    notes.push(`${line === undefined ? '' : `${line}행: `}YAML 파싱 오류 — ${err.message.split('\n')[0]}`);
  }
  if (doc.errors.length > 0) return { notes };
  const value = doc.toJS() as unknown;
  notes.push(...schemaViolations(spec.schema, value));
  return notes.length > 0 ? { notes } : { value: normalizeProse(spec.schema as ShapeSchema, value) as T, notes: [] };
};

type ShapeSchema = {
  type?: string;
  description?: string;
  enum?: string[];
  properties?: Record<string, ShapeSchema>;
  items?: ShapeSchema;
  required?: string[];
  // 원고 인용처럼 글자 그대로 보존해야 하는 필드 — 산문 개행 정규화에서 제외된다.
  verbatim?: boolean;
};

// 폭 맞춤 개행 접기. 블록 스칼라에서 모델이 줄 폭을 맞추려 넣는 단일 개행(실측 60~70자)이
// 파싱된 값에 그대로 남아 뷰어에 노출된다 — 단일 개행은 공백으로 접고, 빈 줄로 나눈 문단만
// 개행으로 유지한다. 인용(verbatim) 필드는 손대지 않는다: 앵커 대조가 원문 그대로를 요구한다.
const unwrapProse = (s: string): string =>
  s
    .split(/\n{2,}/)
    .map((paragraph) =>
      paragraph
        .split('\n')
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .join(' '),
    )
    .filter((paragraph) => paragraph.length > 0)
    .join('\n\n');

const normalizeProse = (schema: ShapeSchema, value: unknown): unknown => {
  if (schema.verbatim) return value;
  if (typeof value === 'string') return unwrapProse(value);
  if (Array.isArray(value)) return value.map((v) => normalizeProse(schema.items ?? {}, v));
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, normalizeProse(schema.properties?.[k] ?? {}, v)]));
  }
  return value;
};

const typeLabel = (schema: ShapeSchema): string => {
  if (schema.enum) return schema.enum.join('|');
  switch (schema.type) {
    case 'string': {
      return '문자열';
    }
    case 'number':
    case 'integer': {
      return '숫자';
    }
    case 'boolean': {
      return '불리언';
    }
    default: {
      return schema.type ?? '';
    }
  }
};

const shapeLines = (schema: ShapeSchema, indent: string, out: string[]): void => {
  for (const [key, child] of Object.entries(schema.properties ?? {})) {
    const note = child.description ? `  # ${child.description}` : '';
    if (child.type === 'object') {
      out.push(`${indent}${key}:${note}`);
      shapeLines(child, `${indent}  `, out);
    } else if (child.type === 'array') {
      out.push(`${indent}${key}:${note}`);
      const item = child.items ?? {};
      if (item.type === 'object') {
        const inner: string[] = [];
        shapeLines(item, `${indent}    `, inner);
        // 배열 항목의 첫 필드에 '- '를 붙여 YAML 목록 모양을 그대로 보여준다.
        if (inner.length > 0) {
          out.push(`${indent}  - ${inner[0].slice(indent.length + 4)}`, ...inner.slice(1));
        }
      } else {
        out.push(`${indent}  - ${typeLabel(item)}`);
      }
    } else {
      out.push(`${indent}${key}: ${typeLabel(child)}${note}`);
    }
  }
};

// 스키마에서 필드 설명이 붙은 YAML 스켈레톤을 렌더한다. 도구 스키마의 필드 설명이 모델에게
// 닿지 않으므로 그 안내를 여기서 복원한다 — 손으로 쓴 사본이 아니라 스키마에서 파생되므로
// 어긋나지 않는다.
export const renderShape = (schema: unknown): string => {
  const out: string[] = [];
  shapeLines(schema as ShapeSchema, '', out);
  return out.join('\n');
};

// 확정 파일 상단의 자기 서술 헤더 — 전 줄이 YAML 주석이라 파싱을 깨지 않고, 이 파일을
// 색인에서 발견한 후속 스테이지가 구조를 스스로 해석하게 한다.
export const finalizeHeader = (spec: OutputSpec, label: string): string =>
  [
    `# ${label} — 이 파일의 구조 (# 뒤는 각 필드의 뜻):`,
    ...renderShape(spec.schema)
      .split('\n')
      .map((line) => `# ${line}`),
  ].join('\n');

// 색인은 워크스페이스의 현재 파일 목록이다 — 후속 스테이지가 이전 산출물을 발견하는 경로.
export const deliverableGuide = (deliverable: Deliverable, index: { path: string; description: string }[]): string =>
  [
    '## 워크스페이스 규약',
    `산출물은 ${deliverable.label} — 아래 선언된 output/ 경로에 YAML 파일로 만든다. write로 쓰고, edit(old_string→new_string, 정확 일치 한 곳)로 고치고, read로 확인한다. scratch/ 아래는 자유 작업 메모 공간이다(검사 없음). 산출물을 저장할 때마다 검사 노트가 돌아온다 — 노트를 해소한 뒤 ${deliverable.submitName}(path)로 제출하면 접수된다.`,
    '',
    '읽을 수 있는 파일:',
    ...index.map((f) => `- ${f.path}${f.description ? ` — ${f.description}` : ''}`),
    '',
    '규칙:',
    '- 산문·인용 필드는 짧아도 반드시 |- 블록 스칼라로 쓴다 — 인용에는 콜론·따옴표가 흔해 인라인으로 쓰면 파싱이 깨진다:',
    '  observation: |-',
    '    걸린 내용을 그대로 쓴다.',
    '- 인용은 원고에서 글자 그대로 복사한다 — 블록 스칼라 안에서는 따옴표도 그대로 둔다.',
    '- 산문 필드 안에서 줄 폭을 맞추려 개행하지 마라 — 한 문단은 한 줄로 쓰고, 문단을 나눌 때만 빈 줄을 둔다. 폭 맞춤 개행은 저장 때 공백으로 접힌다.',
    '- 서로 독립적인 수정 여러 건은 한 턴에 edit를 여러 개 함께 호출한다 — 호출한 순서대로 적용되므로 나눠 부를 이유가 없다.',
    `- 작성이 끝났으면 마지막 write/edit와 같은 턴에 ${deliverable.submitName}까지 함께 호출한다 — 검사에 걸리면 제출만 반려되고 노트가 돌아오므로, 저장 결과를 확인하려고 턴을 나누지 마라.`,
    '- 검사 노트가 남아 있으면 제출은 반려된다. 노트가 가리키는 자리만 고치면 된다 — 멀쩡한 내용을 줄이거나 단순화하지 마라.',
    ...Object.entries(deliverable.outputs).flatMap(([path, spec]) => [
      '',
      `## ${path} — ${spec.description}`,
      '구조 (# 뒤는 각 필드의 작성 지침):',
      renderShape(spec.schema),
    ]),
  ].join('\n');

export const deliverableTools = (deliverable: Deliverable): Anthropic.Messages.Tool[] => [
  ...fileTools(),
  {
    name: deliverable.submitName,
    description: deliverable.submitDescription,
    input_schema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: `확정할 산출물 경로. 선언된 경로: ${declaredPaths(deliverable)}` },
      },
      required: ['path'],
      additionalProperties: false,
    },
    strict: true,
  } as Anthropic.Messages.Tool,
];
