import { EntityLinkKey, RootFieldKey } from './types';
import {
  deepMerge,
  entries,
  getCompatibleTypes,
  isEntityLink,
  isKeyableEntity,
  isScalar,
  makeEntityKey,
  makeFieldKey,
  wrapScalarValue,
} from './utils';
import type { ArtifactSchema, FragmentSpreadSelection, InlineFragmentSelection, ObjectFieldSelection, Selection } from '../../types';
import type { Data, Fields, Storage, Variables } from './types';

export const normalize = (schema: ArtifactSchema, variables: Variables, data: Data): Storage => {
  const storage = {} as Storage;

  const normalizeField = (
    parent: ObjectFieldSelection | FragmentSpreadSelection | InlineFragmentSelection | null,
    children: Selection[],
    value: unknown,
  ): unknown => {
    if (isScalar(value)) {
      return value;
    }

    if (Array.isArray(value)) {
      return value.map((item) => normalizeField(parent, children, item));
    }

    let fields: Record<string, unknown> = {};

    for (const selection of children) {
      switch (selection.kind) {
        case 'ScalarField':
        case 'EnumField':
        case 'TypenameField': {
          const fieldKey = makeFieldKey(selection, variables);
          const fieldValue = (value as Record<string, unknown>)[selection.alias || selection.name];

          if (fieldValue !== undefined) {
            fields[fieldKey] = selection.kind === 'ScalarField' ? wrapScalarValue(fieldValue) : fieldValue;
          }

          break;
        }

        case 'ObjectField': {
          const fieldKey = makeFieldKey(selection, variables);
          const fieldValue = (value as Record<string, unknown>)[selection.alias || selection.name];

          if (fieldValue !== undefined) {
            fields[fieldKey] = normalizeField(selection, selection.children, fieldValue);
          }

          break;
        }

        case 'FragmentSpread': {
          const result = normalizeField(selection, schema.selections.fragments[selection.name], value);
          fields = deepMerge(fields, result, { arrayStrategy: 'merge' });

          break;
        }

        case 'InlineFragment': {
          const typename = (value as { __typename?: string }).__typename;
          if (!typename) {
            break;
          }

          const compatibleTypes = getCompatibleTypes(selection.type);
          if (compatibleTypes.includes(typename)) {
            const result = normalizeField(selection, selection.children, value);
            fields = deepMerge(fields, result, { arrayStrategy: 'merge' });
          }

          break;
        }
      }
    }

    if (isEntityLink(fields)) {
      const entityKey = fields[EntityLinkKey];
      storage[entityKey] = deepMerge(
        storage[entityKey] || {},
        Object.fromEntries(entries(fields).filter(([key]) => key !== EntityLinkKey)),
        { arrayStrategy: 'replace' },
      );
      return { [EntityLinkKey]: entityKey };
    } else if (isKeyableEntity(fields)) {
      const entityKey = makeEntityKey({ __typename: fields.__typename ?? parent?.type.name, id: fields.id });
      storage[entityKey] = deepMerge(storage[entityKey] || {}, fields, { arrayStrategy: 'replace' });
      return { [EntityLinkKey]: entityKey };
    }

    return fields;
  };

  storage[RootFieldKey] = normalizeField(null, schema.selections.operation, data) as Fields;
  return storage;
};
