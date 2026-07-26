// tool 스키마는 모델이 지켜주지 않아도 아무 신호가 나지 않는다. 실제로 두 번 당했다.
// ① 총평의 feedbackIndexes가 5편 중 2편에서 통째로 빠졌다 — 화면에 링크가 안 뜨고서야 드러났다.
// ② SURVEY가 deliberateStyles를 배열이 아니라 JSON 문자열로 반환해 문서 3편이 실패했다.
//    필수 필드는 있었으므로 존재 검사만으로는 걸리지 않았고, 워크플로 본문의 전개 연산자에서 터졌다.
// 그래서 존재와 타입을 함께 본다.

type Schema = {
  type?: string;
  required?: string[];
  properties?: Record<string, Schema>;
  items?: Schema;
};

const typeName = (value: unknown): string => {
  if (Array.isArray(value)) return 'array';
  if (value === null) return 'null';
  return typeof value;
};

const matchesType = (expected: string, value: unknown): boolean => {
  switch (expected) {
    case 'array': {
      return Array.isArray(value);
    }
    case 'object': {
      return typeof value === 'object' && value !== null && !Array.isArray(value);
    }
    case 'integer':
    case 'number': {
      return typeof value === 'number';
    }
    case 'string':
    case 'boolean': {
      return typeof value === expected;
    }
    default: {
      // 모르는 타입은 판정하지 않는다 — 스키마에 없는 제약을 지어내지 않는다.
      return true;
    }
  }
};

const walk = (schema: Schema, value: unknown, path: string, out: string[]): void => {
  const label = path || '(최상위)';

  if (schema.type && !matchesType(schema.type, value)) {
    out.push(`${label}: ${schema.type}가 와야 하는데 ${typeName(value)}입니다`);
    return;
  }

  if (schema.type === 'array') {
    if (!Array.isArray(value) || !schema.items) return;
    for (const [i, item] of value.entries()) {
      walk(schema.items, item, `${path}[${i}]`, out);
    }
    return;
  }

  if (schema.type !== 'object' && !schema.properties) return;
  if (!value || typeof value !== 'object' || Array.isArray(value)) return;
  const record = value as Record<string, unknown>;

  for (const key of schema.required ?? []) {
    if (record[key] === undefined) out.push(`${path ? `${path}.${key}` : key}: 빠졌습니다`);
  }
  for (const [key, child] of Object.entries(schema.properties ?? {})) {
    if (record[key] === undefined) continue;
    walk(child, record[key], path ? `${path}.${key}` : key, out);
  }
};

// 스키마를 어긴 자리를 사람이 읽을 문장으로 돌려준다. 배열 원소는 인덱스까지 붙여 어느 항목인지 짚는다.
export const schemaViolations = (schema: unknown, value: unknown): string[] => {
  const out: string[] = [];
  walk((schema ?? {}) as Schema, value, '', out);
  return out;
};
