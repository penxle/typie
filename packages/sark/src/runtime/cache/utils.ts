import stringify from 'fast-json-stable-stringify';
import { rapidhash } from 'rapidhash-js';
import { match } from 'ts-pattern';
import { EntityLinkKey } from './types';
import type {
  ArtifactSchema,
  CompositeType,
  EnumFieldSelection,
  ObjectFieldSelection,
  ScalarFieldSelection,
  TypenameFieldSelection,
  Value,
} from '../../types';
import type { DependencyKey, EntityKey, EntityLink, FieldKey, KeyableEntity, QueryKey, Scalar, Variables } from './types';

export const isKeyableEntity = (data: unknown): data is KeyableEntity => typeof data === 'object' && data !== null && 'id' in data;

export const makeEntityKey = (entity: KeyableEntity): EntityKey => `${entity.__typename}:${entity.id}`;

export const isEntityLink = (value: unknown): value is EntityLink => typeof value === 'object' && value !== null && EntityLinkKey in value;

export const isScalar = (value: unknown): value is Scalar => {
  const type = typeof value;
  return value === null || value === undefined || type === 'string' || type === 'number' || type === 'boolean';
};

export const makeDependencyKey = (entityKey: EntityKey, fieldKey?: FieldKey): DependencyKey => `${entityKey}:${fieldKey ?? '*'}`;

export const makeFieldKey = (
  selection: TypenameFieldSelection | ScalarFieldSelection | EnumFieldSelection | ObjectFieldSelection,
  variables: Variables,
): FieldKey => {
  if (selection.arguments.length === 0) {
    return selection.name;
  }

  const resolved = Object.fromEntries(selection.arguments.map((arg) => [arg.name, resolveValue(arg.value, variables)]));
  return `${selection.name}$${hashVariables(resolved)}`;
};

export const makeQueryKey = (schema: ArtifactSchema, variables: Variables): QueryKey => {
  return `${schema.name}$${hashVariables(variables)}`;
};

export const makeFieldKeyWithArgs = (field: string, args?: Variables): FieldKey => {
  if (!args || Object.keys(args).length === 0) {
    return field;
  }
  return `${field}$${hashVariables(args)}`;
};

const hashVariables = (variables: Variables) => {
  return rapidhash(stringify(variables)).toString(16);
};

const resolveValue = (value: Value, variables: Variables): unknown => {
  return match(value)
    .with({ kind: 'Variable' }, ({ name }) => variables[name])
    .with({ kind: 'List' }, ({ values }) => values.map((v) => resolveValue(v, variables)))
    .with({ kind: 'Object' }, ({ fields }) => Object.fromEntries(fields.map((field) => [field.name, resolveValue(field.value, variables)])))
    .with({ kind: 'Scalar' }, ({ value }) => value)
    .exhaustive();
};

export const getCompatibleTypes = (type: CompositeType) => {
  return match(type)
    .with({ kind: 'Object' }, (t) => [t.name])
    .with({ kind: 'Interface' }, (t) => [t.name, ...t.implementations])
    .with({ kind: 'Union' }, (t) => [...t.members])
    .exhaustive();
};

export const entries = <T extends object>(obj: T): [keyof T, T[keyof T]][] => {
  return Reflect.ownKeys(obj).map((key) => [key as keyof T, obj[key as keyof T]]);
};

export const deepMerge = <A, B, T extends A & B = A & B>(
  target: A,
  source: B,
  { arrayStrategy }: { arrayStrategy: 'replace' | 'merge' },
): T => {
  if (!source || typeof source !== 'object' || !target || typeof target !== 'object') return source as unknown as T;

  const isPlainObject = (val: unknown): val is Record<string, unknown> => val !== null && typeof val === 'object' && !Array.isArray(val);

  const mergeValues = (a: unknown, b: unknown) => {
    if (isPlainObject(a) && isPlainObject(b)) {
      return deepMerge(a, b, { arrayStrategy });
    }

    if (Array.isArray(a) && Array.isArray(b) && arrayStrategy === 'merge') {
      const result = [...a];

      b.forEach((item, i) => {
        result[i] = i < a.length && isPlainObject(a[i]) && isPlainObject(item) ? deepMerge(a[i], item, { arrayStrategy }) : item;
      });

      return result;
    }

    return b;
  };

  if (Array.isArray(source)) {
    return Array.isArray(target) && arrayStrategy === 'merge' ? (mergeValues(target, source) as unknown as T) : (source as unknown as T);
  }

  if (Array.isArray(target)) {
    return source as unknown as T;
  }

  const result = { ...target } as T;

  entries(source).forEach(([key, value]) => {
    result[key as keyof T] = mergeValues(result[key as keyof T], value) as T[keyof T];
  });

  return result as unknown as T;
};
