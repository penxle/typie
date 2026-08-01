import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { SubscriptionState } from '@typie/lib/enums';

export const load = async (event) => {
  const { url } = event;
  const page = Number(url.searchParams.get('page')) || 1;
  const state = (url.searchParams.get('state') as SubscriptionState | undefined) || undefined;

  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminSubscriptions_Query($state: SubscriptionState, $offset: Int!, $limit: Int!) {
          adminSubscriptions(state: $state, offset: $offset, limit: $limit) {
            totalCount

            subscriptions {
              id
              state
              startsAt
              expiresAt

              plan {
                id
                name
              }

              user {
                id
                name
              }
            }
          }
        }
      `),
      {
        state,
        offset: (page - 1) * ADMIN_ITEMS_PER_PAGE,
        limit: ADMIN_ITEMS_PER_PAGE,
      },
    ),
  };
};
