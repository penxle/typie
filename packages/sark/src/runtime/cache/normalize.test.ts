import { describe, expect, test } from 'vitest';
import { normalize } from './normalize';
import { makeArtifactSchema } from './tests/utils';

describe('normalize', () => {
  test('기본 스칼라 필드를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        Symbol(ROOT): {
          "value": "hello",
        },
      }
    `);
  });

  test('null 스칼라 필드를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        Symbol(ROOT): {
          "value": null,
        },
      }
    `);
  });

  test('ID로 엔티티를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('중첩된 객체 필드를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ... on A {
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
        data: {
          __typename: 'B',
          text: 'Some text',
        },
      },
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "data": {
            "__typename": "B",
            "text": "Some text",
          },
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('스칼라 배열을 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "data": {
            "__typename": "B",
            "tags": [
              "tag1",
              "tag2",
              "tag3",
            ],
          },
          "id": "1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('객체 배열을 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "list": [
            {
              Symbol(LINK): "A:2",
            },
            {
              Symbol(LINK): "A:3",
            },
          ],
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "name": "Entity 2",
        },
        "A:3": {
          "__typename": "A",
          "id": "3",
          "name": "Entity 3",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('빈 객체 배열을 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "list": [],
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('Enum 필드를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "kind": "X",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('Float 스칼라를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            ... on A {
              num
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        num: 42.5,
      },
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "num": 42.5,
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('Date 스칼라를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "time": "2023-01-01T00:00:00Z",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('필드 별칭을 사용해 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('변수를 사용한 쿼리를 정규화한다', () => {
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

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const variables = { id: '1' };

    const storage = normalize(schema, variables, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('자체 참조 필드를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "ref": {
            Symbol(LINK): "A:2",
          },
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "name": "Referenced Entity",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('깊게 중첩된 객체를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "ref": {
            Symbol(LINK): "A:2",
          },
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "ref": {
            Symbol(LINK): "A:3",
          },
        },
        "A:3": {
          "__typename": "A",
          "id": "3",
          "name": "Deeply Nested Entity",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('여러 결과를 포함하는 쿼리를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          find(filter: "test") {
            __typename
            ... on A {
              id
              name
            }
            ... on B {
              text
            }
          }
        }
      `,
    });

    const data = {
      find: [
        { __typename: 'A', id: '1', name: 'Entity 1' },
        { __typename: 'B', text: 'Just text' },
        { __typename: 'A', id: '2', name: 'Entity 2' },
      ],
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "name": "Entity 2",
        },
        Symbol(ROOT): {
          "find$6d26e463b36d050a": [
            {
              Symbol(LINK): "A:1",
            },
            {
              "__typename": "B",
              "text": "Just text",
            },
            {
              Symbol(LINK): "A:2",
            },
          ],
        },
      }
    `);
  });

  test('null 값이 포함된 객체를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "data": null,
          "id": "1",
          "name": "Entity 1",
          "num": null,
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('undefined 필드가 있는 객체를 정규화한다', () => {
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
      },
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('프래그먼트 스프레드를 사용한 쿼리를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
            ...EntityFragment
          }
        }
      `,
      fragments: [
        /* GraphQL */ `
          fragment EntityFragment on Entity {
            __typename
            id
            time
          }
        `,
      ],
    });

    const data = {
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        time: '2023-05-01T12:00:00Z',
      },
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
          "time": "2023-05-01T12:00:00Z",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('여러 프래그먼트를 사용한 쿼리를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "kind": "Y",
          "name": "Entity 1",
          "num": 42,
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('모든 가능한 필드를 포함한 타입 A 쿼리를 정규화한다', () => {
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
          { __typename: 'A', id: '3', name: 'List Item 1' },
          { __typename: 'A', id: '4', name: 'List Item 2' },
        ],
        data: {
          __typename: 'B',
          text: 'Nested text',
          tags: ['tag1', 'tag2'],
        },
      },
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "data": {
            "__typename": "B",
            "tags": [
              "tag1",
              "tag2",
            ],
            "text": "Nested text",
          },
          "id": "1",
          "kind": "X",
          "list": [
            {
              Symbol(LINK): "A:3",
            },
            {
              Symbol(LINK): "A:4",
            },
          ],
          "name": "Complete Entity",
          "num": 123.45,
          "ref": {
            Symbol(LINK): "A:2",
          },
          "time": "2023-05-01T12:00:00Z",
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "name": "Referenced Entity",
        },
        "A:3": {
          "__typename": "A",
          "id": "3",
          "name": "List Item 1",
        },
        "A:4": {
          "__typename": "A",
          "id": "4",
          "name": "List Item 2",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('find 쿼리 결과를 정규화한다', () => {
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

    const data = {
      find: [
        { __typename: 'A', id: '1', name: 'Entity A1', kind: 'X' },
        { __typename: 'B', text: 'Entity B1', tags: ['b1'] },
        { __typename: 'A', id: '2', name: 'Entity A2', kind: 'Y' },
      ],
    };

    const variables = { filter: 'test', kind: 'X' };

    const storage = normalize(schema, variables, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "kind": "X",
          "name": "Entity A1",
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "kind": "Y",
          "name": "Entity A2",
        },
        Symbol(ROOT): {
          "find$89ba49cf4d6820cb": [
            {
              Symbol(LINK): "A:1",
            },
            {
              "__typename": "B",
              "tags": [
                "b1",
              ],
              "text": "Entity B1",
            },
            {
              Symbol(LINK): "A:2",
            },
          ],
        },
      }
    `);
  });

  test('비어있는 find 쿼리 결과를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          find(filter: "empty") {
            __typename
            ... on A {
              id
              name
            }
            ... on B {
              text
            }
          }
        }
      `,
    });

    const data = {
      find: [],
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        Symbol(ROOT): {
          "find$7632e79bbe61e2f6": [],
        },
      }
    `);
  });

  test('ID 없이 타입 A를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            ... on A {
              name
              num
            }
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'A',
        name: 'No ID Entity',
        num: 42,
      },
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            "__typename": "A",
            "name": "No ID Entity",
            "num": 42,
          },
        },
      }
    `);
  });

  test('필드 선택 없이 get 쿼리를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery($id: ID!) {
          get(id: $id) {
            __typename
          }
        }
      `,
    });

    const data = {
      get: {
        __typename: 'Entity',
      },
    };

    const variables = { id: '1' };

    const storage = normalize(schema, variables, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            "__typename": "Entity",
          },
        },
      }
    `);
  });

  test('여러 루트 필드를 포함한 쿼리를 정규화한다', () => {
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
      find: [{ __typename: 'A', id: '2', name: 'Found Entity' }],
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "name": "Found Entity",
        },
        Symbol(ROOT): {
          "find$6d26e463b36d050a": [
            {
              Symbol(LINK): "A:2",
            },
          ],
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
          "value": "root value",
        },
      }
    `);
  });

  test('동일한 엔티티를 여러 번 참조하는 쿼리를 정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          get(id: "1") {
            __typename
            id
            name
          }
          find(filter: "1") {
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
      get: {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
      find: [{ __typename: 'A', id: '1', name: 'Entity 1' }],
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "find$c670fc73f681837f": [
            {
              Symbol(LINK): "A:1",
            },
          ],
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('순환 참조가 있는 객체를 정규화한다', () => {
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

    type CircularEntity = {
      __typename: 'A';
      id: string;
      ref?: CircularEntity;
    };

    const entity1: CircularEntity = {
      __typename: 'A',
      id: '1',
    };

    const entity2: CircularEntity = {
      __typename: 'A',
      id: '2',
    };

    entity1.ref = entity2;
    entity2.ref = entity1;

    const data = {
      get: entity1,
    };

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "id": "1",
          "ref": {
            Symbol(LINK): "A:2",
          },
        },
        "A:2": {
          "__typename": "A",
          "id": "2",
          "ref": {
            Symbol(LINK): "A:1",
          },
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });

  test('변수가 없을 때 기본값을 사용한 쿼리를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:default": {
          "__typename": "A",
          "id": "default",
          "name": "Default Entity",
        },
        Symbol(ROOT): {
          "get$c9c02d9babf17311": {
            Symbol(LINK): "A:default",
          },
        },
      }
    `);
  });

  test('별칭이 있는 중첩 필드를 정규화한다', () => {
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

    const storage = normalize(schema, {}, data);

    expect(storage).toMatchInlineSnapshot(`
      {
        "A:1": {
          "__typename": "A",
          "data": {
            "__typename": "B",
            "text": "Aliased text",
          },
          "id": "1",
          "name": "Entity 1",
        },
        Symbol(ROOT): {
          "get$7f0aa1302c1f9439": {
            Symbol(LINK): "A:1",
          },
        },
      }
    `);
  });
});
