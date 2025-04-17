import { beforeEach, describe, expect, it, vi } from 'vitest';
import { make } from 'wonka';
import { GraphQLError, NetworkError } from '../types';
import { cacheExchange } from './cache';
import { createOperation, runExchange } from './tests/utils';

const mockReadQuery = vi.fn();
const mockWriteQuery = vi.fn();
const mockObserve = vi.fn((schema, variables) => {
  const data = mockReadQuery(schema, variables);
  return make((observer) => {
    observer.next({ data, partial: data === null });
    observer.complete();

    return () => {
      // pass
    };
  });
});

vi.mock('../cache/cache', () => {
  return {
    createCache: () => ({
      readQuery: mockReadQuery,
      writeQuery: mockWriteQuery,
      observe: mockObserve,
    }),
    Cache: vi.fn(),
  };
});

describe('cacheExchange', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('캐시가 없을 때 네트워크 요청을 전달해야 함', async () => {
    mockReadQuery.mockReturnValue(null);

    const operation = createOperation({
      name: 'TestQuery',
      kind: 'query',
      source: 'query TestQuery { test }',
      variables: { id: '1' },
    });

    const result = await runExchange({
      exchange: cacheExchange(),
      operation,
      result: {
        type: 'data',
        operation,
        data: { test: 'response' },
      },
    });

    expect(mockReadQuery).toHaveBeenCalled();

    expect(result).toEqual(
      expect.objectContaining({
        type: 'data',
        data: { test: 'response' },
      }),
    );

    expect(mockWriteQuery).toHaveBeenCalled();
  });

  it('캐시된 데이터가 있을 때 cache-only 정책으로 캐시 데이터를 반환해야 함', async () => {
    const testData = { test: 'cached-data' };
    mockReadQuery.mockReturnValue(testData);

    const queryOperation = createOperation({
      name: 'TestQuery',
      kind: 'query',
      source: 'query TestQuery { test }',
      variables: { id: '1' },
      context: { requestPolicy: 'cache-only' },
    });

    const result = await runExchange({
      exchange: cacheExchange(),
      operation: queryOperation,
    });

    expect(mockReadQuery).toHaveBeenCalled();

    expect(result).toEqual(
      expect.objectContaining({
        type: 'data',
        data: testData,
      }),
    );

    expect(mockWriteQuery).not.toHaveBeenCalled();
  });

  it('mutation 결과를 캐시에 저장해야 함', async () => {
    const mutationData = {
      updateTest: {
        id: '1',
        name: '새 이름',
      },
    };

    const mutationOperation = createOperation({
      name: 'TestMutation',
      kind: 'mutation',
      source: 'mutation TestMutation { updateTest { id name } }',
      variables: { id: '1', name: '새 이름' },
    });

    await runExchange({
      exchange: cacheExchange(),
      operation: mutationOperation,
      result: {
        type: 'data',
        operation: mutationOperation,
        data: mutationData,
      },
    });

    expect(mockWriteQuery).toHaveBeenCalledWith(mutationOperation.schema, mutationOperation.variables, mutationData);

    mockReadQuery.mockReturnValue(mutationData);

    const queryOperation = createOperation({
      name: 'TestQuery',
      kind: 'query',
      source: 'query TestQuery { updateTest { id name } }',
      variables: { id: '1' },
      context: { requestPolicy: 'cache-only' },
    });

    const queryResult = await runExchange({
      exchange: cacheExchange(),
      operation: queryOperation,
    });

    expect(queryResult).toEqual(
      expect.objectContaining({
        type: 'data',
        data: mutationData,
      }),
    );
  });

  it('network-only 정책은 캐시를 무시하고 네트워크 요청을 해야 함', async () => {
    const cachedData = { test: 'cached-data' };
    mockReadQuery.mockReturnValue(cachedData);

    const networkOperation = createOperation({
      name: 'TestQuery',
      kind: 'query',
      source: 'query TestQuery { test }',
      variables: { id: '1' },
      context: { requestPolicy: 'network-only' },
    });

    const networkData = { test: 'network-data' };

    const result = await runExchange({
      exchange: cacheExchange(),
      operation: networkOperation,
      result: {
        type: 'data',
        operation: networkOperation,
        data: networkData,
      },
    });

    expect(result).toEqual(
      expect.objectContaining({
        type: 'data',
        data: networkData,
      }),
    );

    expect(mockWriteQuery).toHaveBeenCalledWith(networkOperation.schema, networkOperation.variables, networkData);

    mockReadQuery.mockReturnValue(networkData);

    const cacheOperation = createOperation({
      name: 'TestQuery',
      kind: 'query',
      source: 'query TestQuery { test }',
      variables: { id: '1' },
      context: { requestPolicy: 'cache-only' },
    });

    const cacheResult = await runExchange({
      exchange: cacheExchange(),
      operation: cacheOperation,
    });

    expect(cacheResult).toEqual(
      expect.objectContaining({
        type: 'data',
        data: networkData,
      }),
    );
  });

  it('GraphQL 에러 응답은 캐시에 저장되지 않아야 함', async () => {
    mockReadQuery.mockReturnValue(null);

    const operation = createOperation({
      name: 'ErrorQuery',
      kind: 'query',
      source: 'query ErrorQuery { errorField }',
      variables: {},
    });

    const graphqlError = new GraphQLError({
      message: 'Field errorField does not exist',
      path: ['errorField'],
      extensions: { code: 'FIELD_NOT_FOUND' },
    });

    const errorResult = await runExchange({
      exchange: cacheExchange(),
      operation,
      result: {
        type: 'error',
        operation,
        error: graphqlError,
      },
    });

    expect(mockWriteQuery).not.toHaveBeenCalled();

    expect(errorResult.type).toBe('error');
    if (errorResult.type === 'error') {
      expect(errorResult.operation).toBe(operation);
      expect(errorResult.error).toBe(graphqlError);
    }

    const cacheOperation = createOperation({
      name: 'ErrorQuery',
      kind: 'query',
      source: 'query ErrorQuery { errorField }',
      variables: {},
      context: { requestPolicy: 'cache-first' },
    });

    const cacheResult = await runExchange({
      exchange: cacheExchange(),
      operation: cacheOperation,
      result: {
        type: 'error',
        operation: cacheOperation,
        error: graphqlError,
      },
    });

    expect(cacheResult.type).toBe('error');
    if (cacheResult.type === 'error') {
      expect(cacheResult.operation).toBe(cacheOperation);
      expect(cacheResult.error).toBe(graphqlError);
    }
  });

  it('네트워크 에러는 캐시에 저장되지 않아야 함', async () => {
    mockReadQuery.mockReturnValue(null);

    const operation = createOperation({
      name: 'NetworkErrorQuery',
      kind: 'query',
      source: 'query NetworkErrorQuery { field }',
      variables: {},
    });

    const networkError = new NetworkError({
      message: '네트워크 연결 실패',
      statusCode: 503,
    });

    const errorResult = await runExchange({
      exchange: cacheExchange(),
      operation,
      result: {
        type: 'error',
        operation,
        error: networkError,
      },
    });

    expect(mockWriteQuery).not.toHaveBeenCalled();

    expect(errorResult.type).toBe('error');
    if (errorResult.type === 'error') {
      expect(errorResult.operation).toBe(operation);
      expect(errorResult.error).toBe(networkError);
    }

    const cacheOperation = createOperation({
      name: 'NetworkErrorQuery',
      kind: 'query',
      source: 'query NetworkErrorQuery { field }',
      variables: {},
      context: { requestPolicy: 'cache-first' },
    });

    const cacheResult = await runExchange({
      exchange: cacheExchange(),
      operation: cacheOperation,
      result: {
        type: 'error',
        operation: cacheOperation,
        error: networkError,
      },
    });

    expect(cacheResult.type).toBe('error');
    if (cacheResult.type === 'error') {
      expect(cacheResult.operation).toBe(cacheOperation);
      expect(cacheResult.error).toBe(networkError);
    }
  });

  it('subscription 타입 요청은 캐시를 확인하지 않고 바로 다음 exchange로 전달해야 함', async () => {
    const operation = createOperation({
      name: 'TestSubscription',
      kind: 'subscription',
      source: 'subscription TestSubscription { onUpdate { id value } }',
      variables: {},
    });

    const subscriptionData = { onUpdate: { id: '1', value: '새 값' } };

    const result = await runExchange({
      exchange: cacheExchange(),
      operation,
      result: {
        type: 'data',
        operation,
        data: subscriptionData,
      },
    });

    expect(result).toEqual(
      expect.objectContaining({
        type: 'data',
        operation,
        data: subscriptionData,
      }),
    );

    expect(mockReadQuery).not.toHaveBeenCalled();
    expect(mockWriteQuery).toHaveBeenCalled();
  });
});
