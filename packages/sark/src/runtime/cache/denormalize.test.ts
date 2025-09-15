import { describe, expect, test } from 'vitest';
import { denormalize } from './denormalize';
import { makeArtifactSchema, makeRootFieldKey } from './tests/utils';
import { EntityLinkKey, RootFieldKey } from './types';
import type { StorageKey } from './types';

describe('denormalize', () => {
  test('ID로 정규화된 엔티티를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
      }
    `);
  });

  test('기본 스칼라 필드를 역정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          value
        }
      `,
    });

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: 'hello',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "value": "hello",
      }
    `);
  });

  test('null 스칼라 필드를 역정규화한다', () => {
    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query TestQuery {
          value
        }
      `,
    });

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: null,
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "value": null,
      }
    `);
  });

  test('중첩된 객체 필드를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        data: {
          __typename: 'B',
          text: 'Some text',
        },
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "data": {
            "__typename": "B",
            "text": "Some text",
          },
          "id": "1",
          "name": "Entity 1",
        },
      }
    `);
  });

  test('스칼라 배열을 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        data: {
          __typename: 'B',
          tags: ['tag1', 'tag2', 'tag3'],
        },
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
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
      }
    `);
  });

  test('객체 배열을 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        list: [
          {
            [EntityLinkKey]: 'A:2',
          },
          {
            [EntityLinkKey]: 'A:3',
          },
        ],
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        name: 'Entity 2',
      },
      'A:3': {
        __typename: 'A',
        id: '3',
        name: 'Entity 3',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "list": [
            {
              "__typename": "A",
              "id": "2",
              "name": "Entity 2",
            },
            {
              "__typename": "A",
              "id": "3",
              "name": "Entity 3",
            },
          ],
        },
      }
    `);
  });

  test('빈 객체 배열을 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        list: [],
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "list": [],
        },
      }
    `);
  });

  test('Enum 필드를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        kind: 'X',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "kind": "X",
        },
      }
    `);
  });

  test('Float 스칼라를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        num: 42.5,
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "num": 42.5,
        },
      }
    `);
  });

  test('Date 스칼라를 역정규화한다', () => {
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
    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        time: date,
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "time": "2023-01-01T00:00:00Z",
        },
      }
    `);
  });

  test('변수를 사용한 쿼리를 역정규화한다', () => {
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
    const fieldKey = makeRootFieldKey(schema, variables);

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const { data } = denormalize(schema, variables, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
      }
    `);
  });

  test('필드 별칭을 사용해 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "entityId": "1",
          "entityName": "Entity 1",
        },
      }
    `);
  });

  test('자체 참조 필드를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        ref: {
          [EntityLinkKey]: 'A:2',
        },
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        name: 'Referenced Entity',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "ref": {
            "__typename": "A",
            "id": "2",
            "name": "Referenced Entity",
          },
        },
      }
    `);
  });

  test('깊게 중첩된 객체를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        ref: {
          [EntityLinkKey]: 'A:2',
        },
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        ref: {
          [EntityLinkKey]: 'A:3',
        },
      },
      'A:3': {
        __typename: 'A',
        id: '3',
        name: 'Deeply Nested Entity',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "ref": {
            "__typename": "A",
            "id": "2",
            "ref": {
              "__typename": "A",
              "id": "3",
              "name": "Deeply Nested Entity",
            },
          },
        },
      }
    `);
  });

  test('여러 결과를 포함하는 쿼리를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: [
          {
            [EntityLinkKey]: 'A:1',
          },
          {
            __typename: 'B',
            text: 'Just text',
          },
          {
            [EntityLinkKey]: 'A:2',
          },
        ],
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        name: 'Entity 2',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "find": [
          {
            "__typename": "A",
            "id": "1",
            "name": "Entity 1",
          },
          {
            "__typename": "B",
            "text": "Just text",
          },
          {
            "__typename": "A",
            "id": "2",
            "name": "Entity 2",
          },
        ],
      }
    `);
  });

  test('프래그먼트 스프레드를 사용한 쿼리를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
      }
    `);
  });

  test('여러 루트 필드를 포함한 쿼리를 역정규화한다', () => {
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

    const valueFieldKey = makeRootFieldKey(schema, {}, 0);
    const getFieldKey = makeRootFieldKey(schema, {}, 1);
    const findFieldKey = makeRootFieldKey(schema, {}, 2);

    const storage = {
      [RootFieldKey]: {
        [valueFieldKey]: 'root value',
        [getFieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
        [findFieldKey]: [
          {
            [EntityLinkKey]: 'A:2',
          },
        ],
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        name: 'Found Entity',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "find": [
          {
            "__typename": "A",
            "id": "2",
            "name": "Found Entity",
          },
        ],
        "get": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
        "value": "root value",
      }
    `);
  });

  test('null 값이 포함된 객체를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        num: null,
        data: null,
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "data": null,
          "id": "1",
          "name": "Entity 1",
          "num": null,
        },
      }
    `);
  });

  test('여러 프래그먼트를 사용한 쿼리를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        num: 42,
        kind: 'Y',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "kind": "Y",
          "name": "Entity 1",
          "num": 42,
        },
      }
    `);
  });

  test('find 쿼리 결과를 역정규화한다', () => {
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
    const fieldKey = makeRootFieldKey(schema, variables);

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: [
          {
            [EntityLinkKey]: 'A:1',
          },
          {
            __typename: 'B',
            tags: ['b1'],
            text: 'Entity B1',
          },
          {
            [EntityLinkKey]: 'A:2',
          },
        ],
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity A1',
        kind: 'X',
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        name: 'Entity A2',
        kind: 'Y',
      },
    };

    const { data } = denormalize(schema, variables, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "find": [
          {
            "__typename": "A",
            "id": "1",
            "kind": "X",
            "name": "Entity A1",
          },
          {
            "__typename": "B",
            "tags": [
              "b1",
            ],
            "text": "Entity B1",
          },
          {
            "__typename": "A",
            "id": "2",
            "kind": "Y",
            "name": "Entity A2",
          },
        ],
      }
    `);
  });

  test('비어있는 find 쿼리 결과를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: [],
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "find": [],
      }
    `);
  });

  test('ID 없이 인라인 저장된 객체를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          __typename: 'A',
          name: 'No ID Entity',
          num: 42,
        },
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "name": "No ID Entity",
          "num": 42,
        },
      }
    `);
  });

  test('동일한 엔티티를 여러 번 참조하는 쿼리를 역정규화한다', () => {
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

    const getFieldKey = makeRootFieldKey(schema, {}, 0);
    const findFieldKey = makeRootFieldKey(schema, {}, 1);

    const storage = {
      [RootFieldKey]: {
        [getFieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
        [findFieldKey]: [
          {
            [EntityLinkKey]: 'A:1',
          },
        ],
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "find": [
          {
            "__typename": "A",
            "id": "1",
            "name": "Entity 1",
          },
        ],
        "get": {
          "__typename": "A",
          "id": "1",
          "name": "Entity 1",
        },
      }
    `);
  });

  test('순환 참조가 있는 객체를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        ref: {
          [EntityLinkKey]: 'A:2',
        },
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        ref: {
          [EntityLinkKey]: 'A:1',
        },
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "1",
          "ref": {
            "__typename": "A",
            "id": "2",
            "ref": {
              "__typename": "A",
              "id": "1",
            },
          },
        },
      }
    `);
  });

  test('변수 기본값을 사용한 쿼리를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:default',
        },
      },
      'A:default': {
        __typename: 'A',
        id: 'default',
        name: 'Default Entity',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
          "__typename": "A",
          "id": "default",
          "name": "Default Entity",
        },
      }
    `);
  });

  test('별칭이 있는 중첩 필드를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
        data: {
          __typename: 'B',
          text: 'Aliased text',
        },
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "entity": {
          "__typename": "A",
          "entityData": {
            "__typename": "B",
            "dataText": "Aliased text",
          },
          "entityId": "1",
          "entityName": "Entity 1",
        },
      }
    `);
  });

  test('참조된 엔티티가 없을 때 null을 반환한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": null,
      }
    `);
  });

  test('모든 가능한 필드를 포함한 타입 A 쿼리를 역정규화한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Complete Entity',
        num: 123.45,
        time: '2023-05-01T12:00:00Z',
        kind: 'X',
        ref: {
          [EntityLinkKey]: 'A:2',
        },
        list: [
          {
            [EntityLinkKey]: 'A:3',
          },
          {
            [EntityLinkKey]: 'A:4',
          },
        ],
        data: {
          __typename: 'B',
          text: 'Nested text',
          tags: ['tag1', 'tag2'],
        },
      },
      'A:2': {
        __typename: 'A',
        id: '2',
        name: 'Referenced Entity',
      },
      'A:3': {
        __typename: 'A',
        id: '3',
        name: 'List Item 1',
      },
      'A:4': {
        __typename: 'A',
        id: '4',
        name: 'List Item 2',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "get": {
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
              "__typename": "A",
              "id": "3",
              "name": "List Item 1",
            },
            {
              "__typename": "A",
              "id": "4",
              "name": "List Item 2",
            },
          ],
          "name": "Complete Entity",
          "num": 123.45,
          "ref": {
            "__typename": "A",
            "id": "2",
            "name": "Referenced Entity",
          },
          "time": "2023-05-01T12:00:00Z",
        },
      }
    `);
  });

  test('accessor 콜백 함수를 호출한다', () => {
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

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'A:1',
        },
      },
      'A:1': {
        __typename: 'A',
        id: '1',
        name: 'Entity 1',
      },
    };

    const accessedEntities: { entityKey: StorageKey; fieldKey: string }[] = [];
    const accessor = (entityKey: StorageKey, fieldKey: string) => {
      accessedEntities.push({ entityKey, fieldKey });
    };

    denormalize(schema, {}, storage, accessor);

    expect(accessedEntities).toMatchInlineSnapshot(`
      [
        {
          "entityKey": "A:1",
          "fieldKey": "*",
        },
        {
          "entityKey": "A:1",
          "fieldKey": "__typename",
        },
        {
          "entityKey": "A:1",
          "fieldKey": "id",
        },
        {
          "entityKey": "A:1",
          "fieldKey": "name",
        },
      ]
    `);
  });

  test('레이아웃과 프래그먼트가 동일한 객체에 다른 필드를 요청할 때 모든 필드가 반환되어야 한다', () => {
    const fragments = [
      /* GraphQL */ `
        fragment UserFragment on User {
          __typename
          id
          sites {
            __typename
            id
          }
        }
      `,
    ];

    const schema = makeArtifactSchema({
      operation: /* GraphQL */ `
        query DashboardLayout {
          me {
            __typename
            id
            sites {
              __typename
              id
              name
            }
            ...UserFragment
          }
        }
      `,
      fragments,
    });

    const fieldKey = makeRootFieldKey(schema, {});

    const storage = {
      [RootFieldKey]: {
        [fieldKey]: {
          [EntityLinkKey]: 'User:1',
        },
      },
      'User:1': {
        __typename: 'User',
        id: '1',
        sites: [
          {
            [EntityLinkKey]: 'Site:1',
          },
          {
            [EntityLinkKey]: 'Site:2',
          },
        ],
      },
      'Site:1': {
        __typename: 'Site',
        id: '1',
        name: 'Site 1',
        url: 'https://site1.com',
      },
      'Site:2': {
        __typename: 'Site',
        id: '2',
        name: 'Site 2',
        url: 'https://site2.com',
      },
    };

    const { data } = denormalize(schema, {}, storage);

    expect(data).toMatchInlineSnapshot(`
      {
        "me": {
          "__typename": "User",
          "id": "1",
          "sites": [
            {
              "__typename": "Site",
              "id": "1",
              "name": "Site 1",
            },
            {
              "__typename": "Site",
              "id": "2",
              "name": "Site 2",
            },
          ],
        },
      }
    `);
  });
});
