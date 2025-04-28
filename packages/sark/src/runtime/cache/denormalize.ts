import { EntityLinkKey, RootFieldKey } from './types';
import { deepMerge, getCompatibleTypes, isEntityLink, isKeyableEntity, isScalar, makeEntityKey, makeFieldKey } from './utils';
import type { ArtifactSchema, FragmentSpreadSelection, InlineFragmentSelection, ObjectFieldSelection, Selection } from '../../types';
import type { Data, EntityKey, FieldKey, Storage, Variables } from './types';

export type DenormalizeResult = {
  data: Data;
  partial: boolean;
};

export const denormalize = (
  schema: ArtifactSchema,
  variables: Variables,
  storage: Storage,
  accessor?: (entityKey: EntityKey, fieldKey: FieldKey) => void,
): DenormalizeResult => {
  let partial = false;

  const denormalizeField = (
    parent: ObjectFieldSelection | FragmentSpreadSelection | InlineFragmentSelection | null,
    children: Selection[],
    value: unknown,
  ): unknown => {
    if (isScalar(value)) {
      return value;
    }

    if (Array.isArray(value)) {
      return value.map((item) => denormalizeField(parent, children, item));
    }

    if (isEntityLink(value)) {
      const entityKey = value[EntityLinkKey];
      accessor?.(entityKey, '*');

      const entity = storage[entityKey];
      if (!entity) {
        partial = true;
        return null;
      }

      return denormalizeField(parent, children, entity);
    }

    if (typeof value === 'object') {
      let fields: Record<string, unknown> = {};

      for (const selection of children) {
        switch (selection.kind) {
          case 'ScalarField':
          case 'EnumField':
          case 'TypenameField': {
            const fieldKey = makeFieldKey(selection, variables);

            if (isKeyableEntity(value)) {
              const entityKey = makeEntityKey({ __typename: value.__typename ?? parent?.type.name, id: value.id });
              accessor?.(entityKey, fieldKey);
            }

            const fieldValue = (value as Record<string, unknown>)[fieldKey];
            if (fieldValue === undefined) {
              partial = true;
            } else {
              fields[selection.alias || selection.name] = fieldValue;
            }

            break;
          }

          case 'ObjectField': {
            const fieldKey = makeFieldKey(selection, variables);

            if (isKeyableEntity(value)) {
              const entityKey = makeEntityKey({ __typename: value.__typename ?? parent?.type.name, id: value.id });
              accessor?.(entityKey, fieldKey);
            }

            const fieldValue = (value as Record<string, unknown>)[fieldKey];

            if (fieldValue === undefined) {
              partial = true;
            } else {
              fields[selection.alias || selection.name] = denormalizeField(selection, selection.children, fieldValue);
            }

            break;
          }

          case 'FragmentSpread': {
            const result = denormalizeField(selection, schema.selections.fragments[selection.name], value);
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
              const result = denormalizeField(selection, selection.children, value);
              fields = deepMerge(fields, result, { arrayStrategy: 'merge' });
            }

            break;
          }
        }
      }

      return fields;
    }

    throw new Error('Invalid value');
  };

  const data = denormalizeField(null, schema.selections.operation, storage[RootFieldKey]) as Data;

  return {
    data,
    partial,
  };
};
