import { applyPatchesImmutable, cacheExchange, createClient } from '@mearie/core';
import { filter, map, pipe, subscribe } from '@mearie/core/stream';
import { expect, test } from 'vitest';
import type { Artifact, Exchange, OperationResult, SchemaMeta } from '@mearie/core';

type Selection = Artifact['selections'][number];

const FIELD_COUNT = 4000;
const MAX_STRUCTURAL_UPDATE_MS = 100;

const schema = {
  entities: {
    Child: { keyFields: ['id'] },
    User: { keyFields: ['id'] },
  },
  inputs: {},
  scalars: {},
} satisfies SchemaMeta;

const identitySelections = [
  { kind: 'Field', name: '__typename', type: 'String' },
  { kind: 'Field', name: 'id', type: 'ID' },
] satisfies Selection[];

const childrenSelection = {
  kind: 'Field',
  name: 'children',
  type: 'Child',
  array: true,
  selections: identitySelections,
} satisfies Selection;

const observedArtifact = {
  kind: 'query',
  name: 'ObservedUser',
  body: 'query ObservedUser { user { id } }',
  selections: [
    {
      kind: 'Field',
      name: 'user',
      type: 'User',
      selections: [
        ...identitySelections,
        childrenSelection,
        ...Array.from({ length: FIELD_COUNT }, (_, index) => ({
          kind: 'Field' as const,
          name: `field${index}`,
          type: 'String',
        })),
      ],
    },
  ],
} satisfies Artifact<'query'>;

const updateArtifact = {
  kind: 'query',
  name: 'UpdatedUser',
  body: 'query UpdatedUser { user { id children { id } } }',
  selections: [
    {
      kind: 'Field',
      name: 'user',
      type: 'User',
      selections: [...identitySelections, childrenSelection],
    },
  ],
} satisfies Artifact<'query'>;

const observedUser = Object.fromEntries([
  ['__typename', 'User'],
  ['id', 'user-1'],
  ['children', [{ __typename: 'Child', id: 'child-1' }]],
  ...Array.from({ length: FIELD_COUNT }, (_, index) => [`field${index}`, `value-${index}`]),
]);

const responses: Record<string, unknown> = {
  ObservedUser: { user: observedUser },
  UpdatedUser: { user: { __typename: 'User', id: 'user-1', children: [{ __typename: 'Child', id: 'child-2' }] } },
};

const fixtureExchange: Exchange = () => ({
  name: 'fixture',
  io: (operations) =>
    pipe(
      operations,
      filter((operation) => operation.variant === 'request'),
      map((operation): OperationResult => ({
        operation,
        data: operation.variant === 'request' ? responses[operation.artifact.name] : undefined,
      })),
    ),
});

test('updates a structurally changed subscription without scanning unrelated cache cursors', async () => {
  const client = createClient({ schema, exchanges: [cacheExchange(), fixtureExchange] });
  const emissions: OperationResult[] = [];
  const unsubscribe = pipe(
    client.executeQuery(observedArtifact, {}),
    subscribe({
      next: (result) => {
        emissions.push(result);
      },
    }),
  );

  try {
    await expect.poll(() => emissions.at(-1)?.data).toBeDefined();
    const initialData = emissions.at(-1)?.data;

    const startedAt = performance.now();
    await client.query(updateArtifact, {}, { fetchPolicy: 'network-only' });
    const elapsedMs = performance.now() - startedAt;

    expect(elapsedMs).toBeLessThan(MAX_STRUCTURAL_UPDATE_MS);
    const patches = emissions.at(-1)?.metadata?.cache?.patches;
    expect(patches).toBeDefined();
    expect(applyPatchesImmutable(initialData, patches ?? [])).toMatchObject({
      user: { children: [{ id: 'child-2' }] },
    });
  } finally {
    unsubscribe();
    client.dispose();
  }
});
