import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphQLError, NetworkError } from '../types';
import { loggingExchange } from './logging';
import { createOperation, runExchange } from './tests/utils';

describe('loggingExchange', () => {
  let logs: unknown[] = [];

  beforeEach(() => {
    logs = [];

    vi.spyOn(console, 'log').mockImplementation((...args) => {
      logs.push(args);
    });

    vi.spyOn(console, 'error').mockImplementation((...args) => {
      logs.push(args);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('요청에 대한 로깅을 수행해야 함', async () => {
    const operation = createOperation({
      name: 'Test',
      kind: 'query',
      source: 'query { test }',
      variables: { var: 'value' },
    });

    await runExchange({
      exchange: loggingExchange(),
      operation,
    });

    expect(logs).toContainEqual([
      '[삵] 요청:',
      expect.objectContaining({
        type: 'query',
        name: 'Test',
      }),
    ]);
  });

  it('성공 응답에 대한 로깅을 수행해야 함', async () => {
    const operation = createOperation({
      name: 'Test',
      kind: 'query',
      source: 'query { test }',
      variables: { var: 'value' },
    });

    await runExchange({
      exchange: loggingExchange(),
      operation,
      result: {
        type: 'data',
        operation,
        data: { test: 'success' },
      },
    });

    expect(logs).toContainEqual([
      '[삵] 응답:',
      expect.objectContaining({
        name: 'Test',
      }),
    ]);
  });

  it('GraphQL 에러 응답에 대한 로깅을 수행해야 함', async () => {
    const operation = createOperation({
      name: 'Test',
      kind: 'query',
      source: 'query { testError }',
      variables: { var: 'value' },
    });

    await runExchange({
      exchange: loggingExchange(),
      operation,
      result: {
        type: 'error',
        operation,
        error: new GraphQLError({
          message: '테스트 에러 메시지',
        }),
      },
    });

    expect(logs).toContainEqual([
      '[삵] 오류:',
      expect.objectContaining({
        error: expect.objectContaining({
          message: '테스트 에러 메시지',
        }),
      }),
    ]);
  });

  it('네트워크 에러 응답에 대한 로깅을 수행해야 함', async () => {
    const operation = createOperation({
      name: 'Test',
      kind: 'query',
      source: 'query { testError }',
      variables: { var: 'value' },
    });

    await runExchange({
      exchange: loggingExchange(),
      operation,
      result: {
        type: 'error',
        operation,
        error: new NetworkError({
          message: '네트워크 에러 메시지',
        }),
      },
    });

    expect(logs).toContainEqual([
      '[삵] 오류:',
      expect.objectContaining({
        error: expect.objectContaining({
          message: '네트워크 에러 메시지',
        }),
      }),
    ]);
  });
});
