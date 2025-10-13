import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import * as graphql from 'graphql';
import { getArtifact } from '../../../codegen/artifact';
import { makeFieldKey } from '../utils';
import type { ArtifactSchema, ObjectFieldSelection } from '../../../types';
import type { Variables } from '../types';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const source = await fs.readFile(path.join(__dirname, 'schema.graphql'), 'utf8');
const schema = graphql.buildSchema(source);

export const makeArtifactSchema = <K extends ArtifactSchema['kind'] = ArtifactSchema['kind']>({
  operation: operationSource,
  fragments: fragmentsSource,
}: {
  operation: string;
  fragments?: string[];
}): ArtifactSchema & { kind: K } => {
  const operation = getArtifact(schema, '', operationSource);
  const fragments = (fragmentsSource ?? []).map((source) => getArtifact(schema, '', source));

  return {
    kind: operation.kind as K,
    name: operation.name,
    source: operation.source,
    selections: {
      operation: operation.selections,
      fragments: Object.fromEntries(fragments.map((v) => [v.name, v.selections])),
    },
    meta: operation.meta,
  };
};

export const makeRootFieldKey = (schema: ArtifactSchema, variables: Variables, index?: number) => {
  return makeFieldKey(schema.selections.operation[index ?? 0] as ObjectFieldSelection, variables);
};
