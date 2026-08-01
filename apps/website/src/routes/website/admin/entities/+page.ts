import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { EntityState, EntityType, EntityVisibility } from '@typie/lib/enums';

export const load = async (event) => {
  const { url } = event;
  const page = Number(url.searchParams.get('page')) || 1;
  const search = url.searchParams.get('search') || undefined;
  const type = (url.searchParams.get('type') as EntityType | undefined) || undefined;
  const state = (url.searchParams.get('state') as EntityState | undefined) || undefined;
  const visibility = (url.searchParams.get('visibility') as EntityVisibility | undefined) || undefined;

  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminEntities_Query(
          $search: String
          $type: EntityType
          $state: EntityState
          $visibility: EntityVisibility
          $offset: Int!
          $limit: Int!
        ) {
          adminEntities(search: $search, type: $type, state: $state, visibility: $visibility, offset: $offset, limit: $limit) {
            totalCount

            entities {
              id
              type
              state
              visibility
              createdAt

              user {
                id
                name
              }

              node {
                __typename

                ... on Document {
                  title
                }

                ... on Folder {
                  name
                }
              }
            }
          }
        }
      `),
      {
        search,
        type,
        state,
        visibility,
        offset: (page - 1) * ADMIN_ITEMS_PER_PAGE,
        limit: ADMIN_ITEMS_PER_PAGE,
      },
    ),
  };
};
