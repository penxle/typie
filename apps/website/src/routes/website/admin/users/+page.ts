import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const { url } = event;
  const page = Number(url.searchParams.get('page')) || 1;
  const search = url.searchParams.get('search') || undefined;
  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminUsers_Query($search: String, $offset: Int!, $limit: Int!) {
          adminUsers(search: $search, offset: $offset, limit: $limit) {
            totalCount

            users {
              id
              name
              email
              role
              state
              createdAt
              avatar {
                id
                url
              }
              singleSignOns {
                id
                provider
                email
              }
              subscription {
                id
                state
                plan {
                  id
                  name
                }
              }
              credit
              sites {
                id
              }
              documentCount
              totalCharacterCount
              marketingConsent
              personalIdentity {
                id
              }
              billingKey {
                id
              }
            }
          }
        }
      `),
      {
        search,
        offset: (page - 1) * ADMIN_ITEMS_PER_PAGE,
        limit: ADMIN_ITEMS_PER_PAGE,
      },
    ),
  };
};
