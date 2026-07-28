import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminUserDetail_Query($userId: String!) {
          adminUser(userId: $userId) {
            id
            name
            email
            role
            state
            hasActiveSubscription

            avatar {
              id
              url
            }

            ...AdminUserOverviewTab_user
            ...AdminUserContentsTab_user
            ...AdminUserBillingTab_user
            ...AdminUserSessionsTab_user
          }
        }
      `),
      {
        userId: event.params.userId,
      },
    ),
  };
};
