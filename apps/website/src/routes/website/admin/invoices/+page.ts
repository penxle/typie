import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
import dayjs from 'dayjs';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';
import type { PaymentInvoiceState } from '@typie/lib/enums';

export const load = async (event) => {
  const { url } = event;
  const page = Number(url.searchParams.get('page')) || 1;
  const state = (url.searchParams.get('state') as PaymentInvoiceState | undefined) || undefined;
  const from = url.searchParams.get('from') || undefined;
  const until = url.searchParams.get('until') || undefined;

  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminInvoices_Query($state: PaymentInvoiceState, $from: DateTime, $until: DateTime, $offset: Int!, $limit: Int!) {
          adminInvoices(state: $state, from: $from, until: $until, offset: $offset, limit: $limit) {
            totalCount

            invoices {
              id
              state
              amount
              createdAt

              user {
                id
                name
              }

              subscription {
                id

                plan {
                  id
                  name
                }
              }
            }
          }
        }
      `),
      {
        state,
        from: from ? dayjs.kst(from).startOf('day').toISOString() : undefined,
        until: until ? dayjs.kst(until).endOf('day').toISOString() : undefined,
        offset: (page - 1) * ADMIN_ITEMS_PER_PAGE,
        limit: ADMIN_ITEMS_PER_PAGE,
      },
    ),
  };
};
