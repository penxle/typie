import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphQLError, NetworkError } from '../types';
import { fetchExchange } from './fetch';
import { createOperation, runExchange } from './tests/utils';

const url = 'https://example.com/graphql';
const fetch = vi.fn();

describe('fetchExchange', () => {
  beforeAll(() => {
    vi.stubGlobal('fetch', fetch);
  });

  afterAll(() => {
    vi.unstubAllGlobals();
  });

  beforeEach(() => {
    fetch.mockReset();
  });

  it('요청에 맞는 fetch 호출을 수행해야 함', async () => {
    const operation = createOperation({
      url,
      name: 'Test',
      kind: 'query',
      source: 'query Test { test }',
      variables: { var: 'value' },
    });

    fetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve({ data: { test: 'response' } }),
    });

    const result = await runExchange({
      exchange: fetchExchange(),
      operation,
    });

    expect(result).toEqual(
      expect.objectContaining({
        type: 'data',
        operation,
        data: { test: 'response' },
      }),
    );

    expect(fetch).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledWith(url, {
      method: 'POST',
      body: JSON.stringify({
        operationName: 'Test',
        query: 'query Test { test }',
        variables: { var: 'value' },
      }),
      headers: {
        'Content-Type': 'application/json',
      },
    });
  });

  it('GraphQL 에러 응답을 적절히 처리해야 함', async () => {
    const operation = createOperation({
      name: 'Test',
      kind: 'query',
      source: 'query Test { test }',
      variables: {},
    });

    fetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve({ errors: [{ message: '테스트 GraphQL 에러' }] }),
    });

    const result = await runExchange({
      exchange: fetchExchange(),
      operation,
    });

    expect(result.type).toBe('error');
    if (result.type === 'error') {
      expect(result.operation).toBe(operation);
      expect(result.error).toBeInstanceOf(GraphQLError);
      if (result.error instanceof GraphQLError) {
        expect(result.error.message).toBe('테스트 GraphQL 에러');
      }
    }
  });

  it('네트워크 오류를 적절히 처리해야 함', async () => {
    const operation = createOperation({
      name: 'Test',
      kind: 'query',
      source: 'query Test { test }',
      variables: {},
    });

    fetch.mockRejectedValueOnce(new Error('테스트 네트워크 에러'));

    const result = await runExchange({
      exchange: fetchExchange(),
      operation,
    });

    expect(result.type).toBe('error');
    if (result.type === 'error') {
      expect(result.operation).toBe(operation);
      expect(result.error).toBeInstanceOf(NetworkError);
      if (result.error instanceof NetworkError) {
        expect(result.error.message).toBe('테스트 네트워크 에러');
      }
    }
  });
});
