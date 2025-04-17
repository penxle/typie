import fs from 'node:fs/promises';
import fg from 'fast-glob';
import * as graphql from 'graphql';
import { rapidhash } from 'rapidhash-js';
import { preprocess } from 'svelte/compiler';
import { match } from 'ts-pattern';
import * as AST from '../ast';
import { buildSelections } from '../parser/selection';
import { buildVariables } from '../parser/variables';
import { hasDirective } from '../utils';
import type { Artifact } from '../../types';

export const getSchema = async (schemaPath: string) => {
  try {
    const source = await fs.readFile(schemaPath, 'utf8');
    const schema = graphql.buildSchema(source, { noLocation: true });

    const errors = graphql.validateSchema(schema);
    if (errors.length > 0) {
      throw new Error(errors.map((e) => e.message).join('\n'));
    }

    return {
      schema,
      hash: rapidhash(source),
    };
  } catch (err) {
    throw new Error(`Failed to process schema: ${String(err)}`);
  }
};

export const getArtifact = (schema: graphql.GraphQLSchema, filePath: string, source: string): Artifact => {
  try {
    const document = graphql.parse(source, { noLocation: true });

    const errors = graphql.validate(
      schema,
      document,
      [...graphql.specifiedRules, ...graphql.recommendedRules].filter(
        (rule) => ![graphql.KnownFragmentNamesRule, graphql.NoUnusedFragmentsRule, graphql.KnownDirectivesRule].includes(rule),
      ),
    );

    if (errors.length > 0) {
      throw new Error(errors.map((e) => e.message).join('\n'));
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
      if (hasDirective(definition.directives, 'client')) {
        meta.client = 'true';
      }

      if (!definition.variableDefinitions?.length) {
        meta.inputless = 'true';
      }

      const root = schema.getRootType(definition.operation);
      if (!root) {
        throw new Error(`Expected root type: ${definition.operation}`);
      }

      return {
        name: definition.name.value,
        file: filePath,
        source,
        kind: definition.operation,
        node: definition,
        meta,
        variables: buildVariables(schema, definition.variableDefinitions),
        selections: buildSelections(schema, root, definition.selectionSet),
        hash: rapidhash(source),
      };
    } else if (definition.kind === 'FragmentDefinition') {
      const on = schema.getType(definition.typeCondition.name.value);
      if (!on) {
        throw new Error(`Expected type: ${definition.typeCondition.name.value}`);
      }

      if (!graphql.isCompositeType(on)) {
        throw new Error(`Expected composite type: ${on.name}`);
      }

      const type = match(on)
        .when(graphql.isObjectType, (t) => ({ kind: 'Object' as const, name: t.name }))
        .when(graphql.isInterfaceType, (t) => ({
          kind: 'Interface' as const,
          name: t.name,
          implementations: schema.getPossibleTypes(t).map((t) => t.name),
        }))
        .when(graphql.isUnionType, (t) => ({ kind: 'Union' as const, name: t.name, members: t.getTypes().map((t) => t.name) }))
        .exhaustive();

      return {
        name: definition.name.value,
        kind: 'fragment' as const,
        file: filePath,
        source,
        on: type,
        node: definition,
        meta: {},
        selections: buildSelections(schema, on, definition.selectionSet),
        hash: rapidhash(source),
      };
    } else {
      throw new Error(`Expected definition to be an operation or fragment`);
    }
  } catch (err) {
    throw new Error(`Failed to process artifact: ${String(err)}`);
  }
};

export const getArtifacts = async (schema: graphql.GraphQLSchema, filePath: string): Promise<Artifact[]> => {
  try {
    const source = await fs.readFile(filePath, 'utf8');
    let script: string | null = null;

    await preprocess(source, {
      script: ({ content, attributes }) => {
        if (attributes.lang !== 'ts') {
          return;
        }

        script = content;
      },
    });

    if (!script) {
      return [];
    }

    const program = AST.parse(script);
    const artifacts: Artifact[] = [];

    AST.walk(program, {
      visitCallExpression(p) {
        const { node } = p;

        if (node.callee.type === 'Identifier' && node.callee.name === 'graphql' && node.arguments[0].type === 'TemplateLiteral') {
          const source = node.arguments[0].quasis[0].value.raw;
          try {
            const artifact = getArtifact(schema, filePath, source);
            artifacts.push(artifact);
          } catch (err) {
            console.error('Error processing artifact:', err);
          }
        }

        this.traverse(p);
      },
    });

    return artifacts;
  } catch (err) {
    throw new Error(`Failed to process file: ${String(err)}`);
  }
};

export const getAllArtifacts = async (schema: graphql.GraphQLSchema, projectDir: string) => {
  try {
    const allArtifacts: Artifact[] = [];
    const files = await fg('**/*.svelte', { cwd: projectDir, absolute: true });

    for (const file of files) {
      try {
        const artifacts = await getArtifacts(schema, file);
        allArtifacts.push(...artifacts);
      } catch (err) {
        console.error(`Error processing file ${file}:`, err);
      }
    }

    return allArtifacts;
  } catch (err) {
    throw new Error(`Failed to gather artifacts: ${String(err)}`);
  }
};
