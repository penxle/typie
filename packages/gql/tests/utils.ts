import fs from 'node:fs/promises';
import path from 'node:path';
import * as graphql from 'graphql';
import { match } from 'ts-pattern';
import { buildArtifacts } from '../src/artifact';
import type { FragmentArtifact, OperationArtifact, StoreSchema } from '../src/types';

const content = await fs.readFile(path.join('./tests/schema.graphql'), 'utf8');
const schema = graphql.buildSchema(content);

export const buildArtifact = (query: string) => {
  const document = graphql.parse(query);

  const operations: Omit<OperationArtifact, 'selections' | 'variables'>[] = [];
  const fragments: Omit<FragmentArtifact, 'selections'>[] = [];

  for (const definition of document.definitions) {
    if (definition.kind === 'OperationDefinition') {
      operations.push({
        name: '',
        kind: definition.operation,
        file: '',
        source: '',
        node: definition,
        meta: {},
      });
    } else if (definition.kind === 'FragmentDefinition') {
      const on = schema.getType(definition.typeCondition.name.value);
      if (!on) {
        throw new Error(`Expected type: ${definition.typeCondition.name.value}`);
      }

      if (!graphql.isCompositeType(on)) {
        throw new Error(`Expected composite type: ${on.name}`);
      }

      fragments.push({
        name: definition.name.value,
        kind: 'fragment',
        file: '',
        source: '',
        on: match(on)
          .when(graphql.isObjectType, (t) => ({
            kind: 'Object' as const,
            name: t.name,
          }))
          .when(graphql.isInterfaceType, (t) => ({
            kind: 'Interface' as const,
            name: t.name,
            implementations: schema.getPossibleTypes(t).map((t) => t.name),
          }))
          .when(graphql.isUnionType, (t) => ({
            kind: 'Union' as const,
            name: t.name,
            members: t.getTypes().map((t) => t.name),
          }))
          .exhaustive(),
        node: definition,
        meta: {},
      });
    }
  }

  return buildArtifacts(schema, operations, fragments);
};

export const buildStoreSchema = (query: string) => {
  const { artifacts } = buildArtifact(query);

  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  const operation = artifacts.find((v) => v.kind !== 'fragment')!;
  const fragments = artifacts.filter((v) => v.kind === 'fragment');

  const storeSchema: StoreSchema = {
    kind: operation.kind,
    name: operation.name,
    source: operation.source,
    selections: {
      operation: operation.selections,
      fragments: Object.fromEntries(fragments.map((v) => [v.name, v.selections])),
    },
    meta: operation.meta,
  };

  return storeSchema;
};
