import { describe, expect, test, vi } from 'vitest';
import { pipe, subscribe } from 'wonka';
import { createCache } from './cache';
import { makeArtifactSchema } from './tests/utils';

describe('Cache', () => {
  test('createCache는 빈 캐시를 생성한다', () => {
    const cache = createCache();
    expect(cache).toBeDefined();
  });

  test('존재하지 않는 쿼리를 읽으면 null을 반환한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const result = cache.readQuery(schema, {});

    expect(result).toBeNull();
  });

  test('clear를 사용해 전체 캐시를 초기화한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    cache.writeQuery(schema, {}, data);

    cache.clear();

    const result = cache.readQuery(schema, {});

    expect(result).toBeNull();
  });

  test('기본 스칼라 필드를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          value
        }
      `,
    });

    const data = {
      value: 'hello',
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('null 스칼라 필드를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          value
        }
      `,
    });

    const data = {
      value: null,
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('ID로 엔티티를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('Enum 필드를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              kind
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        kind: 'X',
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('Date 스칼라를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              time
            }
          }
        }
      `,
    });

    const date = '2023-01-01T00:00:00Z';
    const data = {
      get: {
        __typename: 'A',
        id: '1',
        time: date,
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('null 값이 포함된 객체를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
              num
              data {
                __typename
                text
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        num: null,
        data: null,
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('스칼라 배열을 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              data {
                __typename
                tags
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        data: {
          __typename: 'B',
          tags: ['tag1', 'tag2', 'tag3'],
        },
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('객체 배열을 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              list {
                __typename
                id
                name
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        list: [
          { __typename: 'A', id: '2', name: 'Entity 2' },
          { __typename: 'A', id: '3', name: 'Entity 3' },
        ],
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('빈 객체 배열을 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              list {
                __typename
                id
                name
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        list: [],
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('변수를 사용한 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery($id: ID!) {
          get(id: $id) {
            __typename
            id
            name
          }
        }
      `,
    });

    const variables = { id: '1' };
    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    cache.writeQuery(schema, variables, data);
    const result = cache.readQuery(schema, variables);

    expect(result).toEqual(data);
  });

  test('필드 별칭을 사용한 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            entityId: id
            entityName: name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        entityId: '1',
        entityName: 'Entity 1',
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('변수 기본값을 사용한 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery($id: ID! = "default") {
          get(id: $id) {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: 'default',
        name: 'Default Entity',
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('동일한 쿼리를 다른 변수로 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery($id: ID!) {
          get(id: $id) {
            __typename
            id
            name
          }
        }
      `,
    });

    const data1 = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const data2 = {
      get: {
        __typename: 'A',
        id: '2',
        name: 'Entity 2',
      },
    };

    const variables1 = { id: '1' };
    const variables2 = { id: '2' };

    cache.writeQuery(schema, variables1, data1);
    cache.writeQuery(schema, variables2, data2);

    const result1 = cache.readQuery(schema, variables1);
    const result2 = cache.readQuery(schema, variables2);

    expect(result1).toEqual(data1);
    expect(result2).toEqual(data2);
  });

  test('find 쿼리 결과를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery($filter: String!, $kind: Kind) {
          find(filter: $filter, kind: $kind) {
            __typename
            ... on A {
              id
              name
              kind
            }
            ... on B {
              text
              tags
            }
          }
        }
      `,
    });

    const variables = { filter: 'test', kind: 'X' };
    const data = {
      find: [
        {
          __typename: 'A',
          id: '1',
          name: 'Entity A1',
          kind: 'X',
        },
        {
          __typename: 'B',
          text: 'Entity B1',
          tags: ['b1'],
        },
        {
          __typename: 'A',
          id: '2',
          name: 'Entity A2',
          kind: 'Y',
        },
      ],
    };

    cache.writeQuery(schema, variables, data);
    const result = cache.readQuery(schema, variables);

    expect(result).toEqual(data);
  });

  test('자체 참조 필드를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              ref {
                __typename
                id
                name
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        ref: {
          __typename: 'A',
          id: '2',
          name: 'Referenced Entity',
        },
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('깊게 중첩된 객체를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              ref {
                __typename
                id
                ... on A {
                  ref {
                    __typename
                    id
                    name
                  }
                }
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        ref: {
          __typename: 'A',
          id: '2',
          ref: {
            __typename: 'A',
            id: '3',
            name: 'Deeply Nested Entity',
          },
        },
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('별칭이 있는 중첩 필드를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          entity: get(id: "1") {
            __typename
            entityId: id
            entityName: name
            ... on A {
              entityData: data {
                __typename
                dataText: text
              }
            }
          }
        }
      `,
    });

    const data = {
      entity: {
        __typename: 'A',
        entityId: '1',
        entityName: 'Entity 1',
        entityData: {
          __typename: 'B',
          dataText: 'Aliased text',
        },
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('중첩된 엔티티를 쓰고 읽을 수 있다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
              list {
                __typename
                id
                name
                ... on A {
                  num
                  kind
                }
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: '부모 엔티티',
        list: [
          {
            __typename: 'A',
            id: '101',
            name: '첫 번째 자식',
            num: 42,
            kind: 'X',
          },
          {
            __typename: 'A',
            id: '102',
            name: '두 번째 자식',
            num: 24,
            kind: 'Y',
          },
        ],
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('프래그먼트 스프레드를 사용한 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            ...EntityFragment
          }
        }
      `,
      fragments: [
        /* GraphQL */ `
          fragment EntityFragment on Entity {
            __typename
            id
            name
          }
        `,
      ],
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('여러 프래그먼트를 사용한 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            ...EntityFragment
            ... on A {
              ...TypeAFragment
            }
          }
        }
      `,
      fragments: [
        /* GraphQL */ `
          fragment EntityFragment on Entity {
            __typename
            id
            name
          }
        `,
        /* GraphQL */ `
          fragment TypeAFragment on A {
            __typename
            num
            kind
          }
        `,
      ],
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        num: 42,
        kind: 'Y',
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('여러 루트 필드를 포함한 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          value
          get(id: "1") {
            __typename
            id
            name
          }
          find(filter: "test") {
            __typename
            ... on A {
              id
              name
            }
          }
        }
      `,
    });

    const data = {
      value: 'root value',
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
      find: [
        {
          __typename: 'A',
          id: '2',
          name: 'Found Entity',
        },
      ],
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('유니온 타입 데이터를 처리할 수 있다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          find(filter: "test") {
            __typename
            ... on A {
              id
              name
              kind
            }
            ... on B {
              text
              tags
            }
          }
        }
      `,
    });

    const data = {
      find: [
        {
          __typename: 'A',
          id: '1',
          name: '엔티티 A',
          kind: 'X',
        },
        {
          __typename: 'B',
          text: '텍스트 B',
          tags: ['테스트', '유니온'],
        },
      ],
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('writeQuery 후 readQuery가 동일한 엔티티를 반환한다', () => {
    const cache = createCache();
    const getSchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query GetQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const findSchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query FindQuery {
          find(filter: "test") {
            __typename
            ... on A {
              id
              name
            }
          }
        }
      `,
    });

    const getData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const findData = {
      find: [
        {
          __typename: 'A',
          id: '1',
          name: 'Entity 1',
        },
      ],
    };

    cache.writeQuery(getSchema, {}, getData);

    cache.writeQuery(findSchema, {}, findData);

    const getResult = cache.readQuery(getSchema, {});
    const findResult = cache.readQuery(findSchema, {});

    expect(getResult).toEqual(getData);
    expect(findResult).toEqual(findData);
  });

  test('모든 가능한 필드를 포함한 타입 A 쿼리를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
              num
              time
              kind
              ref {
                __typename
                id
                name
              }
              list {
                __typename
                id
                name
              }
              data {
                __typename
                text
                tags
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Complete Entity',
        num: 123.45,
        time: '2023-05-01T12:00:00Z',
        kind: 'X',
        ref: {
          __typename: 'A',
          id: '2',
          name: 'Referenced Entity',
        },
        list: [
          {
            __typename: 'A',
            id: '3',
            name: 'List Item 1',
          },
          {
            __typename: 'A',
            id: '4',
            name: 'List Item 2',
          },
        ],
        data: {
          __typename: 'B',
          text: 'Nested text',
          tags: ['tag1', 'tag2'],
        },
      },
    };

    cache.writeQuery(schema, {}, data);
    const result = cache.readQuery(schema, {});

    expect(result).toEqual(data);
  });

  test('writeFragment를 사용해 엔티티를 업데이트한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial name',
      },
    };

    cache.writeQuery(schema, {}, data);

    cache.writeFragment('A:1', {
      __typename: 'A',
      name: 'Updated name',
    });

    const result = cache.readQuery(schema, {});

    expect(result).toEqual({
      get: {
        __typename: 'A',
        id: '1',
        name: 'Updated name',
      },
    });
  });

  test('invalidate를 사용해 엔티티 필드를 무효화한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    cache.writeQuery(schema, {}, data);

    cache.invalidate('A:1', 'name');

    const result = cache.readQuery(schema, {});

    expect(result).toBeNull();
  });

  test('invalidate를 사용해 전체 엔티티를 무효화한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    cache.writeQuery(schema, {}, data);

    cache.invalidate('A:1');

    const result = cache.readQuery(schema, {});

    expect(result).toBeNull();
  });

  test('중복 데이터를 쓸 때 깊은 병합이 적용된다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
              num
              data {
                __typename
                text
                tags
              }
            }
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: '초기 엔티티',
        num: 50,
        data: {
          __typename: 'B',
          text: '초기 텍스트',
          tags: ['태그1', '태그2'],
        },
      },
    };

    cache.writeQuery(schema, {}, initialData);

    const updateData = {
      get: {
        __typename: 'A',
        id: '1',
        data: {
          __typename: 'B',
          text: '업데이트된 텍스트',
        },
      },
    };

    cache.writeQuery(schema, {}, updateData);

    const result = cache.readQuery(schema, {});

    expect(result).toEqual({
      get: {
        __typename: 'A',
        id: '1',
        name: '초기 엔티티',
        num: 50,
        data: {
          __typename: 'B',
          text: '업데이트된 텍스트',
          tags: ['태그1', '태그2'],
        },
      },
    });
  });

  test('observe를 사용해 쿼리 결과 변경을 구독한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
      },
    };

    cache.writeQuery(schema, {}, initialData);

    const callback = vi.fn();
    const source = cache.observe(schema, {});
    const subscription = pipe(source, subscribe(callback));

    expect(callback).toHaveBeenCalledTimes(1);
    expect(callback).toHaveBeenCalledWith({ data: initialData, partial: false });

    const updatedData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Updated Name',
      },
    };

    cache.writeQuery(schema, {}, updatedData);

    expect(callback).toHaveBeenCalledTimes(2);
    expect(callback).toHaveBeenCalledWith({ data: updatedData, partial: false });

    subscription.unsubscribe();

    cache.writeQuery(
      schema,
      {},
      {
        get: {
          __typename: 'A',
          id: '1',
          name: 'Name After Unsubscribe',
        },
      },
    );

    expect(callback).toHaveBeenCalledTimes(2);
  });

  test('동일한 쿼리에 대해 여러 구독자가 알림을 받는다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: '엔티티 1',
      },
    };

    cache.writeQuery(schema, {}, data);

    const callback1 = vi.fn();
    const callback2 = vi.fn();
    const callback3 = vi.fn();

    const source1 = cache.observe(schema, {});
    const source2 = cache.observe(schema, {});
    const source3 = cache.observe(schema, {});

    pipe(source1, subscribe(callback1));
    pipe(source2, subscribe(callback2));
    pipe(source3, subscribe(callback3));

    callback1.mockClear();
    callback2.mockClear();
    callback3.mockClear();

    cache.writeFragment('A:1', {
      __typename: 'A',
      name: '업데이트된 엔티티',
    });

    expect(callback1).toHaveBeenCalledTimes(1);
    expect(callback2).toHaveBeenCalledTimes(1);
    expect(callback3).toHaveBeenCalledTimes(1);
  });

  test('특정 필드만 업데이트 시 해당 필드에 의존하는 쿼리만 갱신된다', () => {
    const cache = createCache();

    const nameOnlySchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query NameQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const kindOnlySchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query KindQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              kind
            }
          }
        }
      `,
    });

    const userData = {
      get: {
        __typename: 'A',
        id: '1',
        name: '엔티티 1',
        kind: 'X',
      },
    };

    cache.writeQuery(nameOnlySchema, {}, userData);
    cache.writeQuery(kindOnlySchema, {}, userData);

    const nameCallback = vi.fn();
    const kindCallback = vi.fn();

    const source1 = cache.observe(nameOnlySchema, {});
    const source2 = cache.observe(kindOnlySchema, {});

    pipe(source1, subscribe(nameCallback));
    pipe(source2, subscribe(kindCallback));

    nameCallback.mockClear();
    kindCallback.mockClear();

    cache.writeFragment('A:1', {
      __typename: 'A',
      name: '업데이트된 엔티티',
    });

    expect(nameCallback).toHaveBeenCalledTimes(1);
    expect(kindCallback).not.toHaveBeenCalled();

    nameCallback.mockClear();
    kindCallback.mockClear();

    cache.writeFragment('A:1', {
      __typename: 'A',
      kind: 'Y',
    });

    expect(nameCallback).not.toHaveBeenCalled();
    expect(kindCallback).toHaveBeenCalledTimes(1);
  });

  test('쿼리 결과 업데이트 시 의존 쿼리가 갱신된다', () => {
    const cache = createCache();
    const fullSchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query FullQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
              num
            }
          }
        }
      `,
    });

    const partialSchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query PartialQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
        num: 10,
      },
    };

    cache.writeQuery(fullSchema, {}, initialData);

    const fullResult1 = cache.readQuery(fullSchema, {});
    const partialResult1 = cache.readQuery(partialSchema, {});

    expect(fullResult1).toEqual(initialData);
    expect(partialResult1).toEqual({
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
      },
    });

    cache.writeQuery(
      partialSchema,
      {},
      {
        get: {
          __typename: 'A',
          id: '1',
          name: 'Updated Name',
        },
      },
    );

    const fullResult2 = cache.readQuery(fullSchema, {});
    const partialResult2 = cache.readQuery(partialSchema, {});

    expect(partialResult2).toEqual({
      get: {
        __typename: 'A',
        id: '1',
        name: 'Updated Name',
      },
    });

    expect(fullResult2).toEqual({
      get: {
        __typename: 'A',
        id: '1',
        name: 'Updated Name',
        num: 10,
      },
    });
  });

  test('순환 참조가 있는 객체를 읽고 쓴다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              ref {
                __typename
                id
                ... on A {
                  ref {
                    __typename
                    id
                  }
                }
              }
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        ref: {
          __typename: 'A',
          id: '2',
          ref: {
            __typename: 'A',
            id: '1',
          },
        },
      },
    };

    cache.writeQuery(schema, {}, data);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const result = cache.readQuery(schema, {}) as any;

    expect(result).toEqual(data);

    expect(result.get.id).toBe('1');
    expect(result.get.__typename).toBe('A');
    expect(result.get.ref.id).toBe('2');
    expect(result.get.ref.__typename).toBe('A');
    expect(result.get.ref.ref.id).toBe('1');
    expect(result.get.ref.ref.__typename).toBe('A');
  });

  test('동일한 ID를 가진 다른 타입의 엔티티를 구분하여 처리할 수 있다', () => {
    const cache = createCache();

    const entitySchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query EntityQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const resultSchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query ResultQuery {
          find(filter: "test") {
            __typename
            ... on B {
              text
              tags
            }
          }
        }
      `,
    });

    const entityData = {
      get: {
        __typename: 'A',
        id: '1',
        name: '엔티티 1',
      },
    };

    const resultData = {
      find: [
        {
          __typename: 'B',
          text: '텍스트 B',
          tags: ['태그1', '태그2'],
        },
      ],
    };

    cache.writeQuery(entitySchema, {}, entityData);
    cache.writeQuery(resultSchema, {}, resultData);

    const entityResult = cache.readQuery(entitySchema, {});
    const resultResult = cache.readQuery(resultSchema, {});

    expect(entityResult).toEqual(entityData);
    expect(resultResult).toEqual(resultData);
  });

  test('동일한 필드를 가진 서로 다른 fragment 사용시 무한 루프가 발생하지 않아야 한다', () => {
    const cache = createCache();

    const mainQuery = makeArtifactSchema({
      operation: /* GraphQL */ `
        query GetUser {
          me {
            __typename
            id
            name
            email
            sites {
              __typename
              id
              name
              url
            }
          }
        }
      `,
    });

    const nameFragment = makeArtifactSchema({
      operation: /* GraphQL */ `
        query NameOnly {
          me {
            __typename
            id
            name
            sites {
              __typename
              id
              name
            }
          }
        }
      `,
    });

    const emailFragment = makeArtifactSchema({
      operation: /* GraphQL */ `
        query EmailOnly {
          me {
            __typename
            id
            email
            sites {
              __typename
              id
              url
            }
          }
        }
      `,
    });

    const variables = {};
    const initialData = {
      me: {
        __typename: 'User',
        id: 'user1',
        name: 'John Doe',
        email: 'john@example.com',
        sites: [
          {
            __typename: 'Site',
            id: 'site1',
            name: 'Site 1',
            url: 'https://site1.com',
          },
          {
            __typename: 'Site',
            id: 'site2',
            name: 'Site 2',
            url: 'https://site2.com',
          },
        ],
      },
    };

    cache.writeQuery(mainQuery, variables, initialData);

    let observerCallCount = 0;
    const observers: { unsubscribe: () => void }[] = [];

    const mainSub = pipe(
      cache.observe(mainQuery, variables),
      subscribe(() => {
        observerCallCount++;
      }),
    );
    observers.push(mainSub);

    const nameSub = pipe(
      cache.observe(nameFragment, variables),
      subscribe(() => {
        observerCallCount++;
      }),
    );
    observers.push(nameSub);

    const emailSub = pipe(
      cache.observe(emailFragment, variables),
      subscribe(() => {
        observerCallCount++;
      }),
    );
    observers.push(emailSub);

    observerCallCount = 0;

    cache.writeQuery(mainQuery, variables, initialData);

    cache.writeQuery(mainQuery, variables, initialData);
    cache.writeQuery(mainQuery, variables, initialData);

    expect(observerCallCount).toBeLessThanOrEqual(3);

    const updatedData = {
      ...initialData,
      me: {
        ...initialData.me,
        name: 'Jane Doe',
      },
    };

    const beforeUpdateCount = observerCallCount;
    cache.writeQuery(mainQuery, variables, updatedData);

    expect(observerCallCount).toBeGreaterThan(beforeUpdateCount);

    observers.forEach((sub) => sub.unsubscribe());
  });

  test('addOptimisticLayer는 즉시 쿼리 결과에 반영된다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
      },
    };

    cache.writeQuery(schema, {}, initialData);

    const optimisticData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Optimistic Name',
      },
    };

    cache.addOptimisticLayer('opt-1', schema, {}, optimisticData);

    const result = cache.readQuery(schema, {});

    expect(result).toEqual(optimisticData);
  });

  test('removeOptimisticLayer는 optimistic data를 제거한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
      },
    };

    cache.writeQuery(schema, {}, initialData);

    const optimisticData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Optimistic Name',
      },
    };

    cache.addOptimisticLayer('opt-1', schema, {}, optimisticData);
    cache.removeOptimisticLayer('opt-1');

    const result = cache.readQuery(schema, {});

    expect(result).toEqual(initialData);
  });

  test('여러 optimistic layers가 올바르게 병합된다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
              num
            }
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
        num: 10,
      },
    };

    cache.writeQuery(schema, {}, initialData);

    cache.addOptimisticLayer(
      'opt-1',
      schema,
      {},
      {
        get: {
          __typename: 'A',
          id: '1',
          name: 'Optimistic Name 1',
        },
      },
    );

    cache.addOptimisticLayer(
      'opt-2',
      schema,
      {},
      {
        get: {
          __typename: 'A',
          id: '1',
          num: 99,
        },
      },
    );

    const result = cache.readQuery(schema, {});

    expect(result).toEqual({
      get: {
        __typename: 'A',
        id: '1',
        name: 'Optimistic Name 1',
        num: 99,
      },
    });
  });

  test('optimistic layer는 observe 구독자에게 알림을 보낸다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
      },
    };

    cache.writeQuery(schema, {}, initialData);

    const callback = vi.fn();
    const source = cache.observe(schema, {});
    pipe(source, subscribe(callback));

    callback.mockClear();

    const optimisticData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Optimistic Name',
      },
    };

    cache.addOptimisticLayer('opt-1', schema, {}, optimisticData);

    expect(callback).toHaveBeenCalledTimes(1);
    expect(callback).toHaveBeenCalledWith({
      data: optimisticData,
      partial: false,
    });

    callback.mockClear();

    cache.removeOptimisticLayer('opt-1');

    expect(callback).toHaveBeenCalledTimes(1);
    expect(callback).toHaveBeenCalledWith({
      data: initialData,
      partial: false,
    });
  });

  test('clearOptimisticLayers는 모든 optimistic layers를 제거한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const initialData = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Initial Name',
      },
    };

    cache.writeQuery(schema, {}, initialData);

    cache.addOptimisticLayer(
      'opt-1',
      schema,
      {},
      {
        get: {
          __typename: 'A',
          id: '1',
          name: 'Optimistic 1',
        },
      },
    );

    cache.addOptimisticLayer(
      'opt-2',
      schema,
      {},
      {
        get: {
          __typename: 'A',
          id: '1',
          name: 'Optimistic 2',
        },
      },
    );

    cache.clearOptimisticLayers();

    const result = cache.readQuery(schema, {});

    expect(result).toEqual(initialData);
  });

  test('optimistic layer는 새로운 엔티티를 추가할 수 있다', () => {
    const cache = createCache();
    const listSchema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query ListQuery {
          find(filter: "test") {
            __typename
            ... on A {
              id
              name
            }
          }
        }
      `,
    });

    const initialData = {
      find: [
        {
          __typename: 'A',
          id: '1',
          name: 'Entity 1',
        },
      ],
    };

    cache.writeQuery(listSchema, {}, initialData);

    const optimisticData = {
      find: [
        {
          __typename: 'A',
          id: '1',
          name: 'Entity 1',
        },
        {
          __typename: 'A',
          id: '2',
          name: 'New Entity',
        },
      ],
    };

    cache.addOptimisticLayer('opt-1', listSchema, {}, optimisticData);

    const result = cache.readQuery(listSchema, {});

    expect(result).toEqual(optimisticData);
  });

  test('optimistic layer의 임시 ID와 실제 response의 ID가 다른 경우를 처리한다', () => {
    const cache = createCache();
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
        }
      `,
    });

    const optimisticData = {
      get: {
        __typename: 'A',
        id: 'temp-id',
        name: 'Optimistic Entity',
      },
    };

    cache.addOptimisticLayer('opt-1', schema, {}, optimisticData);

    const optimisticResult = cache.readQuery(schema, {});
    expect(optimisticResult).toEqual(optimisticData);

    const actualData = {
      get: {
        __typename: 'A',
        id: 'server-123',
        name: 'Optimistic Entity',
      },
    };

    cache.writeQuery(schema, {}, actualData);

    cache.removeOptimisticLayer('opt-1');

    const finalResult = cache.readQuery(schema, {});
    expect(finalResult).toEqual(actualData);
  });
});
