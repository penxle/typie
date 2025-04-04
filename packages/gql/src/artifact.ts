import fs from 'node:fs/promises';
import path from 'node:path';
import fg from 'fast-glob';
import * as graphql from 'graphql';
import { preprocess } from 'svelte/compiler';
import { match } from 'ts-pattern';
import * as AST from './ast';
import { buildSelections } from './parser/selection';
import { buildVariables } from './parser/variables';
import { hasDirective } from './utils';
import type { FragmentArtifact, OperationArtifact } from './types';

export const collectArtifacts = async (root: string) => {
  const files = await fg('**/*.svelte', {
    cwd: root,
    absolute: true,
    ignore: ['node_modules'],
  });

  let schema: graphql.GraphQLSchema;
  try {
    const source = await fs.readFile(path.join(root, 'schema.graphql'), 'utf8');
    schema = graphql.buildSchema(source);
  } catch (err: unknown) {
    throw new Error('Failed to load schema', { cause: err });
  }

  const operations: Omit<OperationArtifact, 'selections' | 'variables'>[] = [];
  const fragments: Omit<FragmentArtifact, 'selections'>[] = [];

  await Promise.all(
    files.map(async (file) => {
      const source = await fs.readFile(file, 'utf8');

      await preprocess(source, {
        script: async ({ content, attributes }) => {
          if (attributes.type !== undefined || attributes.lang !== 'ts') {
            return;
          }

          let program;
          try {
            program = AST.parse(content);
          } catch (err: unknown) {
            throw new Error('Failed to parse script', { cause: err });
          }

          AST.walk(program, {
            visitCallExpression(p) {
              const { node } = p;

              if (node.callee.type === 'Identifier' && node.callee.name === 'graphql' && node.arguments[0].type === 'TemplateLiteral') {
                const source = node.arguments[0].quasis[0].value.raw;

                let document;
                try {
                  document = graphql.parse(source);
                } catch (err: unknown) {
                  throw new Error('Failed to parse document', { cause: err });
                }

                if (document.definitions.length !== 1) {
                  throw new Error('Expected exactly one definition');
                }

                const [definition] = document.definitions;
                if (definition.kind === 'OperationDefinition') {
                  if (!definition.name) {
                    throw new Error('Expected operation to have a name');
                  }

                  const meta: Record<string, string> = {};
                  if (hasDirective(definition.directives, 'manual')) {
                    meta.mode = 'manual';
                  }

                  if (!definition.variableDefinitions?.length) {
                    meta.inputless = 'true';
                  }

                  operations.push({
                    name: definition.name.value,
                    file,
                    source,
                    kind: definition.operation,
                    node: definition,
                    meta,
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
                    file,
                    source,
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

              this.traverse(p);
            },
          });
        },
      });
    }),
  );

  return buildArtifacts(schema, operations, fragments);
};

export const buildArtifacts = (
  schema: graphql.GraphQLSchema,
  operations: Omit<OperationArtifact, 'selections' | 'variables'>[],
  fragments: Omit<FragmentArtifact, 'selections'>[],
) => {
  const operationMap = new Map<string, OperationArtifact>();
  const fragmentMap = new Map<string, FragmentArtifact>();

  for (const fragment of fragments) {
    if (fragmentMap.has(fragment.name)) {
      throw new Error(`Duplicate fragment name: ${fragment.name}`);
    }

    const on = schema.getType(fragment.on.name);
    if (!on) {
      throw new Error(`Expected type: ${fragment.on.name}`);
    }

    if (!graphql.isCompositeType(on)) {
      throw new Error(`Expected composite type: ${on.name}`);
    }

    fragmentMap.set(fragment.name, {
      ...fragment,
      selections: buildSelections(schema, on, fragment.node.selectionSet),
    });
  }

  for (const operation of operations) {
    if (operationMap.has(operation.name)) {
      throw new Error(`Duplicate operation name: ${operation.name}`);
    }

    const root = schema.getRootType(operation.kind as graphql.OperationTypeNode);
    if (!root) {
      throw new Error(`Expected root type: ${operation.kind}`);
    }

    operationMap.set(operation.name, {
      ...operation,
      variables: buildVariables(schema, operation.node.variableDefinitions),
      selections: buildSelections(schema, root, operation.node.selectionSet),
    });
  }

  const sortedOperationMap = new Map([...operationMap.entries()].sort((a, b) => a[0].localeCompare(b[0])));
  const sortedFragmentMap = new Map([...fragmentMap.entries()].sort((a, b) => a[0].localeCompare(b[0])));

  const artifacts = [...sortedOperationMap.values(), ...sortedFragmentMap.values()];

  return {
    schema,
    artifacts,
  };
};
