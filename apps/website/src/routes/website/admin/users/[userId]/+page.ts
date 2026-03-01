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
            createdAt
            avatar {
              id
              url
            }
            sites {
              id
              name
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
              startsAt
              expiresAt
              plan {
                id
                name
                availability
              }
            }
            credit
            personalIdentity {
              id
              name
              birthDate
              gender
              phoneNumber
            }
            marketingConsent
            documentCount
            totalCharacterCount
            billingKey {
              id
              name
            }
          }
        }
      `),
      {
        userId: event.params.userId,
      },
    ),
  };
};
