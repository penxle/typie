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
              currentPeriodEndsAt
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
            billingKey {
              id
              name
            }
            usage {
              totalCharacterCount
            }
            paymentInvoices {
              id
              state
              amount
              dueAt
              createdAt
              subscription {
                id
                plan {
                  id
                  name
                }
              }
              records {
                id
                outcome
                billingAmount
                creditAmount
                data
                createdAt
              }
            }
          }
          adminPrismCredit(userId: $userId) {
            total
            paid
            free
            display
          }
          adminPrismCreditEntries(userId: $userId, limit: 50) {
            id
            kind
            paidDelta
            freeDelta
            key
            note
            createdAt
            actor {
              id
              name
            }
          }
          adminPrismCreditPurchases(userId: $userId) {
            id
            pack
            price
            credits
            bonusCredits
            state
            paymentKey
            paidAt
            createdAt
            refundedAmount
            data
          }
          adminPrismCreditRefunds(userId: $userId) {
            id
            kind
            purchaseId
            amount
            method
            state
            note
            createdAt
            data
            actor {
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
