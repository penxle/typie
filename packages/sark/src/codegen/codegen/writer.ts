import path from 'node:path';
import * as graphql from 'graphql';
import * as R from 'remeda';
import * as AST from '../ast';
import { getReferencedFragments } from '../parser/selection';
import { addIdAndTypenameField, removeDirective, writeFile } from '../utils';
import { buildGraphQLFunctions, buildGraphQLTypes, buildSelectionsTSType, buildVariablesTSType } from './generator';
import type { Artifact, ArtifactSchema, FragmentArtifact, OperationArtifact } from '../../types';

export const writeArtifactAssets = async (outDir: string, schema: graphql.GraphQLSchema, artifacts: Artifact[]) => {
  const operationArtifacts = artifacts.filter((v) => v.kind !== 'fragment') as OperationArtifact[];
  const fragmentArtifacts = artifacts.filter((v) => v.kind === 'fragment') as FragmentArtifact[];
  const fragmentMap = new Map(fragmentArtifacts.map((v) => [v.name, v]));

  for (const operation of operationArtifacts) {
    const fragments = R.uniqueBy(getReferencedFragments(operation.selections, fragmentMap), (v) => v.name);

    const source = [operation.node, ...fragments.map((v) => v.node)]
      .map((v) => graphql.print(addIdAndTypenameField(schema, removeDirective(v, ['required', 'client']))))
      .join('\n\n');

    const storeSchema: ArtifactSchema = {
      kind: operation.kind,
      name: operation.name,
      source,
      selections: {
        operation: operation.selections,
        fragments: Object.fromEntries(fragments.map((v) => [v.name, v.selections])),
      },
      meta: operation.meta,
    };

    const functionName = `create${R.capitalize(operation.kind)}Store`;

    const program = AST.b.program([
      AST.b.importDeclaration.from({
        source: AST.b.stringLiteral('@typie/sark/internal'),
        specifiers: [AST.b.importSpecifier(AST.b.identifier(functionName))],
      }),
      AST.b.exportNamedDeclaration(
        AST.b.tsTypeAliasDeclaration.from({
          id: AST.b.identifier(operation.name),
          typeAnnotation: AST.b.tsTypeLiteral([
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$name'),
              typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsLiteralType(AST.b.stringLiteral(operation.name))),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$kind'),
              typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsLiteralType(AST.b.stringLiteral(operation.kind))),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$input'),
              typeAnnotation: AST.b.tsTypeAnnotation(buildVariablesTSType(operation.variables)),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$output'),
              typeAnnotation: AST.b.tsTypeAnnotation(buildSelectionsTSType(operation.selections)),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$meta'),
              typeAnnotation: AST.b.tsTypeAnnotation(
                AST.b.tsTypeLiteral(
                  Object.entries(operation.meta).map(([key, value]) =>
                    AST.b.tsPropertySignature.from({
                      key: AST.b.identifier(key),
                      typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsLiteralType(AST.b.stringLiteral(value))),
                    }),
                  ),
                ),
              ),
            }),
          ]),
        }),
      ),
      AST.b.variableDeclaration.from({
        kind: 'const',
        declarations: [
          AST.b.variableDeclarator.from({
            id: AST.b.identifier('schema'),
            init: AST.b.callExpression.from({
              callee: AST.b.identifier('JSON.parse'),
              arguments: [AST.b.stringLiteral(JSON.stringify(storeSchema))],
            }),
          }),
        ],
      }),
      AST.b.exportDefaultDeclaration.from({
        declaration: AST.b.functionDeclaration.from({
          id: null,
          params: [],
          body: AST.b.blockStatement([
            AST.b.returnStatement(
              AST.b.callExpression.from({
                callee: AST.b.identifier(functionName),
                arguments: [AST.b.identifier('schema')],
                typeArguments: AST.b.typeParameterInstantiation([AST.b.typeParameter(operation.name)]),
              }),
            ),
          ]),
        }),
      }),
    ]);

    const content = AST.print(program);
    await writeFile(path.join(outDir, `artifacts/operations/${operation.name}.ts`), content);
  }

  for (const fragment of fragmentArtifacts) {
    const program = AST.b.program([
      AST.b.exportNamedDeclaration(
        AST.b.tsTypeAliasDeclaration.from({
          id: AST.b.identifier(fragment.name),
          typeAnnotation: AST.b.tsTypeLiteral([
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$name'),
              typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsLiteralType(AST.b.stringLiteral(fragment.name))),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$kind'),
              typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsLiteralType(AST.b.stringLiteral(fragment.kind))),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$input'),
              typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsTypeLiteral([])),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$output'),
              typeAnnotation: AST.b.tsTypeAnnotation(buildSelectionsTSType(fragment.selections, fragment.on)),
            }),
            AST.b.tsPropertySignature.from({
              key: AST.b.identifier('$meta'),
              typeAnnotation: AST.b.tsTypeAnnotation(
                AST.b.tsTypeLiteral(
                  Object.entries(fragment.meta).map(([key, value]) =>
                    AST.b.tsPropertySignature.from({
                      key: AST.b.identifier(key),
                      typeAnnotation: AST.b.tsTypeAnnotation(AST.b.tsLiteralType(AST.b.stringLiteral(value))),
                    }),
                  ),
                ),
              ),
            }),
          ]),
        }),
      ),
    ]);

    const content = AST.print(program);
    await writeFile(path.join(outDir, `artifacts/fragments/${fragment.name}.ts`), content);
  }
};

export const writePublicAssets = async (outDir: string, artifacts: Artifact[]) => {
  const functions = buildGraphQLFunctions(artifacts);
  await writeFile(path.join(outDir, 'public/functions.d.ts'), AST.print(functions));

  const types = buildGraphQLTypes(artifacts);
  await writeFile(path.join(outDir, 'public/types.d.ts'), AST.print(types));

  const indexTs = AST.b.program([
    AST.b.importDeclaration.from({
      importKind: 'type',
      source: AST.b.stringLiteral('@typie/sark/internal'),
      specifiers: [AST.b.importSpecifier.from({ imported: AST.b.identifier('Cache') })],
    }),
    AST.b.exportNamedDeclaration.from({
      declaration: null,
      specifiers: [
        AST.b.exportSpecifier.from({
          local: AST.b.identifier('FragmentType'),
          exported: AST.b.identifier('FragmentType'),
        }),
        AST.b.exportSpecifier.from({
          local: AST.b.identifier('Optional'),
          exported: AST.b.identifier('Optional'),
        }),
        AST.b.exportSpecifier.from({
          local: AST.b.identifier('List'),
          exported: AST.b.identifier('List'),
        }),
      ],
      source: AST.b.stringLiteral('@typie/sark/internal'),
    }),
    AST.b.exportAllDeclaration(AST.b.stringLiteral('./public/functions')),
    AST.b.exportAllDeclaration(AST.b.stringLiteral('./public/types')),
    AST.b.exportNamedDeclaration.from({
      declaration: AST.b.variableDeclaration.from({
        kind: 'const',
        declarations: [
          AST.b.variableDeclarator.from({
            id: AST.b.identifier.from({
              name: 'cache',
              typeAnnotation: AST.b.tsTypeAnnotation.from({
                typeAnnotation: AST.b.tsTypeReference.from({
                  typeName: AST.b.identifier('Cache'),
                }),
              }),
            }),
          }),
        ],
      }),
    }),
  ]);
  await writeFile(path.join(outDir, 'index.d.ts'), AST.print(indexTs));

  const indexJs = AST.b.program([AST.b.exportAllDeclaration(AST.b.stringLiteral('@typie/sark/internal'))]);
  await writeFile(path.join(outDir, 'index.js'), AST.print(indexJs));
};

export const writeMiscAssets = async (outDir: string) => {
  const client = AST.b.program([
    AST.b.exportNamedDeclaration.from({
      declaration: null,
      specifiers: [
        AST.b.exportSpecifier.from({
          local: AST.b.identifier('default'),
          exported: AST.b.identifier('default'),
        }),
      ],
      source: AST.b.stringLiteral('../src/lib/graphql'),
    }),
  ]);
  await writeFile(path.join(outDir, 'client.js'), AST.print(client));
};

export const writeTypeAssets = async (outDir: string, projectDir: string, artifacts: Artifact[]) => {
  const queries = artifacts.filter((v) => v.kind === 'query' && v.meta.client !== 'true') as OperationArtifact[];

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const typeMap = new Map<string, any[]>();

  for (const query of queries) {
    const basepath = path.relative(projectDir, path.dirname(query.file));
    const filename = path.join(outDir, 'types', basepath, '$graphql.d.ts');
    const eventName = path.basename(query.file).startsWith('+layout') ? 'LayoutLoadEvent' : 'PageLoadEvent';

    const types = typeMap.get(filename) ?? [];

    types.push(
      AST.b.importDeclaration.from({
        importKind: 'type',
        source: AST.b.stringLiteral(
          path.relative(path.dirname(filename), path.join(projectDir, '.svelte-kit/types', basepath, '$types.d.ts')),
        ),
        specifiers: [
          AST.b.importSpecifier.from({
            imported: AST.b.identifier(eventName),
          }),
        ],
      }),
      AST.b.importDeclaration.from({
        importKind: 'type',
        source: AST.b.stringLiteral(path.relative(path.dirname(filename), path.join(outDir, `artifacts/operations/${query.name}`))),
        specifiers: [
          AST.b.importSpecifier.from({
            imported: AST.b.identifier(query.name),
          }),
        ],
      }),
      AST.b.exportNamedDeclaration.from({
        declaration: AST.b.tsTypeAliasDeclaration.from({
          id: AST.b.identifier(`${query.name}_Variables`),
          typeAnnotation: AST.b.tsTypeReference.from({
            typeName: AST.b.identifier('VariablesFn'),
            typeParameters: AST.b.tsTypeParameterInstantiation([
              AST.b.tsTypeReference(AST.b.identifier(eventName)),
              AST.b.tsTypeReference(AST.b.identifier(query.name)),
            ]),
          }),
        }),
      }),
      AST.b.exportNamedDeclaration.from({
        declaration: AST.b.tsTypeAliasDeclaration.from({
          id: AST.b.identifier(`${query.name}_AfterLoad`),
          typeAnnotation: AST.b.tsTypeReference.from({
            typeName: AST.b.identifier('AfterLoadFn'),
            typeParameters: AST.b.tsTypeParameterInstantiation([
              AST.b.tsTypeReference(AST.b.identifier(eventName)),
              AST.b.tsTypeReference(AST.b.identifier(query.name)),
            ]),
          }),
        }),
      }),
      AST.b.exportNamedDeclaration.from({
        declaration: AST.b.tsTypeAliasDeclaration.from({
          id: AST.b.identifier(`${query.name}_OnError`),
          typeAnnotation: AST.b.tsTypeReference.from({
            typeName: AST.b.identifier('OnErrorFn'),
            typeParameters: AST.b.tsTypeParameterInstantiation([AST.b.tsTypeReference(AST.b.identifier(eventName))]),
          }),
        }),
      }),
    );

    typeMap.set(filename, types);
  }

  for (const [filename, types] of typeMap) {
    const program = AST.b.program([
      AST.b.importDeclaration.from({
        importKind: 'type',
        source: AST.b.stringLiteral('@typie/sark/internal'),
        specifiers: [
          AST.b.importSpecifier.from({
            imported: AST.b.identifier('VariablesFn'),
          }),
          AST.b.importSpecifier.from({
            imported: AST.b.identifier('AfterLoadFn'),
          }),
          AST.b.importSpecifier.from({
            imported: AST.b.identifier('OnErrorFn'),
          }),
        ],
      }),
      ...types,
    ]);

    await writeFile(filename, AST.print(program));
  }
};
