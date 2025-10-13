import { beforeEach, describe, expect, it, vi } from 'vitest';
import { make } from 'wonka';
import { makeArtifactSchema } from '../../cache/tests/utils';
import type { GraphQLOperation, OperationResult } from '../../types';
import type { MutationStore } from './mutation';

const mockClient = {
  createOperation: vi.fn(),
  executeOperation: vi.fn(),
};

const mockCache = {
  addOptimisticLayer: vi.fn(),
  removeOptimisticLayer: vi.fn(),
};

vi.mock('../../client/internal', () => ({
  getClient: () => mockClient,
}));

vi.mock('../../cache/cache', () => ({
  createCache: () => mockCache,
}));

const { createMutationStore } = await import('./mutation');

const mockSchema = makeArtifactSchema<'mutation'>({
  operation: /* GraphQL */ `
    mutation SaveEntity($id: ID!, $data: Input!) {
      save(id: $id, data: $data) {
        __typename
        id
        name
      }
    }
  `,
});

type TestMutationInput = { id: string; data: { name?: string } };
type TestMutationOutput = { __typename: string; id: string; name: string };
type TestMutation = MutationStore<{
  $name: string;
  $kind: 'mutation';
  $input: { input: TestMutationInput };
  $output: { save: TestMutationOutput };
  $meta: Record<string, unknown>;
}>;

function createTestMutation(): TestMutation {
  return createMutationStore(mockSchema) as TestMutation;
}

describe('MutationStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('optimisticResponse 없이 mutation을 실행한다', async () => {
    const mutation = createTestMutation();

    const input = { id: '1', data: { name: 'New Name' } };
    const mockOperation: GraphQLOperation = {
      key: 'op-1',
      type: 'mutation',
      schema: mockSchema,
      variables: { input },
      context: {
        url: 'http://localhost:3000/graphql',
        requestPolicy: 'network-only',
        transport: 'fetch',
      },
    };

    mockClient.createOperation.mockReturnValue(mockOperation);

    const mockResult: OperationResult = {
      type: 'data',
      operation: mockOperation,
      data: {
        save: {
          __typename: 'A',
          id: '1',
          name: 'New Name',
        },
      },
    };

    mockClient.executeOperation.mockReturnValue(
      make((observer) => {
        observer.next(mockResult);
        observer.complete();
        return () => {
          // pass
        };
      }),
    );

    const result = await mutation(input);

    expect(result).toEqual({
      __typename: 'A',
      id: '1',
      name: 'New Name',
    });

    expect(mockCache.addOptimisticLayer).not.toHaveBeenCalled();
    expect(mockCache.removeOptimisticLayer).not.toHaveBeenCalled();
  });

  it('optimistic과 함께 mutation을 실행한다', async () => {
    const mutation = createTestMutation();

    const input = { id: '1', data: { name: 'New Name' } };
    const optimistic = {
      __typename: 'A',
      id: '1',
      name: 'New Name',
    };

    const mockOperation: GraphQLOperation = {
      key: 'op-1',
      type: 'mutation',
      schema: mockSchema,
      variables: { input },
      context: {
        url: 'http://localhost:3000/graphql',
        requestPolicy: 'network-only',
        transport: 'fetch',
        optimistic,
      },
    };

    mockClient.createOperation.mockReturnValue(mockOperation);

    const mockResult: OperationResult = {
      type: 'data',
      operation: mockOperation,
      data: {
        save: {
          __typename: 'A',
          id: '1',
          name: 'New Name',
        },
      },
    };

    mockClient.executeOperation.mockReturnValue(
      make((observer) => {
        observer.next(mockResult);
        observer.complete();
        return () => {
          // pass
        };
      }),
    );

    const result = await mutation(input, { optimistic });

    expect(result).toEqual({
      __typename: 'A',
      id: '1',
      name: 'New Name',
    });

    expect(mockCache.addOptimisticLayer).toHaveBeenCalledWith(expect.any(String), mockSchema, { input }, { save: optimistic });

    expect(mockCache.removeOptimisticLayer).toHaveBeenCalledWith(expect.any(String));
  });

  it('mutation 에러 시 optimistic layer를 제거한다', async () => {
    const mutation = createTestMutation();

    const input = { id: '1', data: { name: 'New Name' } };
    const optimistic = {
      __typename: 'A',
      id: '1',
      name: 'New Name',
    };

    const mockOperation: GraphQLOperation = {
      key: 'op-1',
      type: 'mutation',
      schema: mockSchema,
      variables: { input },
      context: {
        url: 'http://localhost:3000/graphql',
        requestPolicy: 'network-only',
        transport: 'fetch',
        optimistic,
      },
    };

    mockClient.createOperation.mockReturnValue(mockOperation);

    const error = new Error('Mutation failed');
    const mockResult: OperationResult = {
      type: 'error',
      operation: mockOperation,
      error,
    };

    mockClient.executeOperation.mockReturnValue(
      make((observer) => {
        observer.next(mockResult);
        observer.complete();
        return () => {
          // pass
        };
      }),
    );

    await expect(mutation(input, { optimistic })).rejects.toThrow('Mutation failed');

    expect(mockCache.addOptimisticLayer).toHaveBeenCalledWith(expect.any(String), mockSchema, { input }, { save: optimistic });

    expect(mockCache.removeOptimisticLayer).toHaveBeenCalledWith(expect.any(String));
  });

  it('mutation 실행 중 예외 발생 시 optimistic layer를 제거한다', async () => {
    const mutation = createTestMutation();

    const input = { id: '1', data: { name: 'New Name' } };
    const optimistic = {
      __typename: 'A',
      id: '1',
      name: 'New Name',
    };

    const mockOperation: GraphQLOperation = {
      key: 'op-1',
      type: 'mutation',
      schema: mockSchema,
      variables: { input },
      context: {
        url: 'http://localhost:3000/graphql',
        requestPolicy: 'network-only',
        transport: 'fetch',
        optimistic,
      },
    };

    mockClient.createOperation.mockReturnValue(mockOperation);

    const error = new Error('Network error');
    mockClient.executeOperation.mockReturnValue(
      make(() => {
        throw error;
      }),
    );

    await expect(mutation(input, { optimistic })).rejects.toThrow('Network error');

    expect(mockCache.addOptimisticLayer).toHaveBeenCalledWith(expect.any(String), mockSchema, { input }, { save: optimistic });

    expect(mockCache.removeOptimisticLayer).toHaveBeenCalledWith(expect.any(String));
  });

  it('동일한 optimistic key를 사용하여 layer를 추가하고 제거한다', async () => {
    const mutation = createTestMutation();

    const input = { id: '1', data: { name: 'New Name' } };
    const optimistic = {
      __typename: 'A',
      id: '1',
      name: 'New Name',
    };

    const mockOperation: GraphQLOperation = {
      key: 'op-1',
      type: 'mutation',
      schema: mockSchema,
      variables: { input },
      context: {
        url: 'http://localhost:3000/graphql',
        requestPolicy: 'network-only',
        transport: 'fetch',
        optimistic,
      },
    };

    mockClient.createOperation.mockReturnValue(mockOperation);

    const mockResult: OperationResult = {
      type: 'data',
      operation: mockOperation,
      data: {
        save: {
          __typename: 'A',
          id: '1',
          name: 'New Name',
        },
      },
    };

    mockClient.executeOperation.mockReturnValue(
      make((observer) => {
        observer.next(mockResult);
        observer.complete();
        return () => {
          // pass
        };
      }),
    );

    await mutation(input, { optimistic });

    expect(mockCache.addOptimisticLayer).toHaveBeenCalledTimes(1);
    expect(mockCache.removeOptimisticLayer).toHaveBeenCalledTimes(1);

    const addCall = mockCache.addOptimisticLayer.mock.calls[0];
    const removeCall = mockCache.removeOptimisticLayer.mock.calls[0];

    expect(addCall[0]).toBe(removeCall[0]);
  });

  it('optimistic response의 id와 실제 response의 id가 다른 경우를 처리한다', async () => {
    const mutation = createTestMutation();

    const input = { id: 'new', data: { name: 'New Name' } };
    const optimistic = {
      __typename: 'A',
      id: 'temp-id',
      name: 'New Name',
    };

    const mockOperation: GraphQLOperation = {
      key: 'op-1',
      type: 'mutation',
      schema: mockSchema,
      variables: { input },
      context: {
        url: 'http://localhost:3000/graphql',
        requestPolicy: 'network-only',
        transport: 'fetch',
        optimistic,
      },
    };

    mockClient.createOperation.mockReturnValue(mockOperation);

    const mockResult: OperationResult = {
      type: 'data',
      operation: mockOperation,
      data: {
        save: {
          __typename: 'A',
          id: 'server-123',
          name: 'New Name',
        },
      },
    };

    mockClient.executeOperation.mockReturnValue(
      make((observer) => {
        observer.next(mockResult);
        observer.complete();
        return () => {
          // pass
        };
      }),
    );

    const result = await mutation(input, { optimistic });

    expect(result).toEqual({
      __typename: 'A',
      id: 'server-123',
      name: 'New Name',
    });

    expect(mockCache.addOptimisticLayer).toHaveBeenCalledWith(expect.any(String), mockSchema, { input }, { save: optimistic });

    expect(mockCache.removeOptimisticLayer).toHaveBeenCalledWith(expect.any(String));
  });
});
