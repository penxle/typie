import recast from 'recast';
import * as AST from '../ast';
import type { Artifact } from '../../types';

export const transformGraphQL = (artifacts: Artifact[], source: string) => {
  let program;
  try {
    program = AST.parse(source);
  } catch {
    return;
  }

  let propsUsed = false;

  AST.walk(program, {
    visitCallExpression(p) {
      const { node } = p;

      if (node.callee.type === 'Identifier' && node.callee.name === 'fragment') {
        p.replace(
          AST.b.callExpression.from({
            callee: AST.b.identifier('$derived'),
            arguments: [node],
          }),
        );

        return false;
      }

      this.traverse(p);
    },
  });

  AST.walk(program, {
    visitCallExpression(p) {
      const { node } = p;

      if (node.callee.type === 'Identifier' && node.callee.name === 'graphql' && node.arguments[0].type === 'TemplateLiteral') {
        const source = node.arguments[0].quasis[0].value.raw;
        const artifact = artifacts.find((artifact) => artifact.source === source);

        if (artifact) {
          if (artifact.kind === 'fragment') {
            p.replace(AST.b.nullLiteral());
          } else if (artifact.kind === 'query' && artifact.meta.client !== 'true') {
            p.replace(
              AST.b.callExpression.from({
                callee: AST.b.identifier('$derived'),
                arguments: [AST.b.identifier(`__sark_props.data.__sark_${artifact.name}`)],
              }),
            );
            propsUsed = true;
          } else {
            p.replace(
              AST.b.callExpression.from({
                callee: AST.b.identifier(`__sark_${artifact.name}`),
                arguments: [],
              }),
            );
            program.body.unshift(
              AST.b.importDeclaration.from({
                source: AST.b.stringLiteral(`$graphql/artifacts/operations/${artifact.name}`),
                specifiers: [
                  AST.b.importDefaultSpecifier.from({
                    local: AST.b.identifier(`__sark_${artifact.name}`),
                  }),
                ],
              }),
            );
          }
        }
      }

      this.traverse(p);
    },
  });

  if (propsUsed) {
    AST.walk(program, {
      visitCallExpression(p) {
        const { node } = p;
        if (node.callee.type === 'Identifier' && node.callee.name === '$props') {
          p.replace(AST.b.identifier('__sark_props'));
        }

        this.traverse(p);
      },
    });

    program.body.unshift(
      AST.b.variableDeclaration.from({
        kind: 'let',
        declarations: [
          AST.b.variableDeclarator.from({
            id: AST.b.identifier('__sark_props'),
            init: AST.b.callExpression.from({
              callee: AST.b.identifier('$props'),
              arguments: [],
            }),
          }),
        ],
      }),
    );
  }

  return AST.print(program);
};

export const transformLoad = (artifacts: Artifact[], source: string, filePath: string) => {
  if (!filePath.endsWith('+page.ts') && !filePath.endsWith('+layout.ts')) {
    return;
  }

  const queries = artifacts.filter(
    (artifact) => artifact.kind === 'query' && artifact.meta.client !== 'true' && artifact.file.startsWith(filePath.replace(/\.ts$/, '')),
  );

  if (queries.length === 0) {
    return;
  }

  let program;
  try {
    program = AST.parse(source);
  } catch {
    return;
  }

  const exportedVariables: Record<string, recast.types.namedTypes.VariableDeclarator> = {};

  AST.walk(program, {
    visitExportNamedDeclaration(p) {
      const { node } = p;

      if (node.declaration?.type === 'VariableDeclaration') {
        for (const declaration of node.declaration.declarations) {
          if (declaration.type === 'VariableDeclarator' && declaration.id.type === 'Identifier') {
            exportedVariables[declaration.id.name] = declaration;
          }
        }
      }

      this.traverse(p);
    },
  });

  const loaders = ['__sark_load'];
  if ('load' in exportedVariables) {
    // @ts-expect-error already checked
    exportedVariables.load.id.name = '__sark_original_load';
    loaders.unshift('__sark_original_load');
  }

  program.body.push(
    AST.b.importDeclaration.from({
      source: AST.b.stringLiteral('@typie/sark/internal'),
      specifiers: [
        AST.b.importSpecifier.from({
          imported: AST.b.identifier('handleError'),
          local: AST.b.identifier('__sark_handleError'),
        }),
      ],
    }),
    ...queries.map((query) =>
      AST.b.importDeclaration.from({
        source: AST.b.stringLiteral(`$graphql/artifacts/operations/${query.name}`),
        specifiers: [
          AST.b.importDefaultSpecifier.from({
            local: AST.b.identifier(`__sark_${query.name}`),
          }),
        ],
      }),
    ),
    AST.b.functionDeclaration.from({
      id: AST.b.identifier('__sark_load'),
      params: [AST.b.identifier('event')],
      async: true,
      body: AST.b.blockStatement([
        AST.b.variableDeclaration.from({
          kind: 'const',
          declarations: queries.map((query) =>
            AST.b.variableDeclarator.from({
              id: AST.b.identifier(query.name),
              init: AST.b.callExpression.from({
                callee: AST.b.identifier(`__sark_${query.name}`),
                arguments: [],
              }),
            }),
          ),
        }),
        AST.b.variableDeclaration.from({
          kind: 'let',
          declarations: queries.map((query) =>
            AST.b.variableDeclarator.from({
              id: AST.b.identifier(`__sark_${query.name}_data`),
            }),
          ),
        }),
        ...queries.map((query) =>
          AST.b.tryStatement.from({
            block: AST.b.blockStatement([
              AST.b.expressionStatement(
                AST.b.assignmentExpression.from({
                  left: AST.b.identifier(`__sark_${query.name}_data`),
                  operator: '=',
                  right: AST.b.awaitExpression(
                    AST.b.callExpression.from({
                      callee: AST.b.memberExpression(AST.b.identifier(query.name), AST.b.identifier('load')),
                      arguments: [
                        `_${query.name}_Variables` in exportedVariables
                          ? AST.b.awaitExpression(
                              AST.b.callExpression.from({
                                callee: AST.b.identifier(`_${query.name}_Variables`),
                                arguments: [AST.b.identifier('event')],
                              }),
                            )
                          : AST.b.nullLiteral(),
                        AST.b.objectExpression([
                          AST.b.objectProperty(
                            AST.b.identifier('fetch'),
                            AST.b.memberExpression(AST.b.identifier('event'), AST.b.identifier('fetch')),
                          ),
                        ]),
                      ],
                    }),
                  ),
                }),
              ),
            ]),
            handler: AST.b.catchClause.from({
              param: AST.b.identifier('error'),
              body: AST.b.blockStatement([
                ...(`_${query.name}_OnError` in exportedVariables
                  ? [
                      AST.b.expressionStatement(
                        AST.b.awaitExpression(
                          AST.b.callExpression.from({
                            callee: AST.b.identifier(`_${query.name}_OnError`),
                            arguments: [
                              AST.b.objectExpression([
                                AST.b.objectProperty(AST.b.identifier('error'), AST.b.identifier('error')),
                                AST.b.objectProperty(AST.b.identifier('event'), AST.b.identifier('event')),
                              ]),
                            ],
                          }),
                        ),
                      ),
                    ]
                  : []),
                AST.b.expressionStatement(
                  AST.b.awaitExpression(
                    AST.b.callExpression.from({
                      callee: AST.b.identifier('__sark_handleError'),
                      arguments: [
                        AST.b.objectExpression([
                          AST.b.objectProperty(AST.b.identifier('error'), AST.b.identifier('error')),
                          AST.b.objectProperty(AST.b.identifier('event'), AST.b.identifier('event')),
                        ]),
                      ],
                    }),
                  ),
                ),
                AST.b.throwStatement(AST.b.identifier('error')),
              ]),
            }),
          }),
        ),
        ...queries
          .filter((query) => `_${query.name}_AfterLoad` in exportedVariables)
          .map((query) =>
            AST.b.expressionStatement(
              AST.b.awaitExpression(
                AST.b.callExpression.from({
                  callee: AST.b.identifier(`_${query.name}_AfterLoad`),
                  arguments: [
                    AST.b.objectExpression([
                      AST.b.objectProperty(AST.b.identifier('query'), AST.b.identifier(`__sark_${query.name}_data`)),
                      AST.b.objectProperty(AST.b.identifier('event'), AST.b.identifier('event')),
                    ]),
                  ],
                }),
              ),
            ),
          ),
        AST.b.returnStatement(
          AST.b.objectExpression(
            queries.map((query) => AST.b.objectProperty(AST.b.identifier(`__sark_${query.name}`), AST.b.identifier(query.name))),
          ),
        ),
      ]),
    }),
    AST.b.exportNamedDeclaration(
      AST.b.functionDeclaration.from({
        id: AST.b.identifier('load'),
        params: [AST.b.identifier('event')],
        async: true,
        body: AST.b.blockStatement([
          AST.b.returnStatement(
            AST.b.objectExpression(
              loaders.map((loader) =>
                AST.b.spreadProperty(
                  AST.b.awaitExpression(
                    AST.b.callExpression.from({
                      callee: AST.b.identifier(loader),
                      arguments: [AST.b.identifier('event')],
                    }),
                  ),
                ),
              ),
            ),
          ),
        ]),
      }),
    ),
  );

  return AST.print(program);
};
