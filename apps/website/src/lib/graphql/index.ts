import { error, redirect } from '@sveltejs/kit';
import { cacheExchange, createClient, errorExchange, fetchExchange, GraphQLError, NetworkError, sseExchange } from '@typie/sark';
import { TypieError } from '@/errors';

// eslint-disable-next-line import/no-default-export
export default createClient({
  url: `/graphql`,
  fetchOptions: {
    credentials: 'include',
  },
  exchanges: [
    errorExchange((error) => {
      if (error instanceof GraphQLError && error.extensions?.type === 'TypieError') {
        return new TypieError({
          code: error.extensions.code as string,
          message: error.message,
          status: error.extensions.status as number,
        });
      }

      return error;
    }),
    cacheExchange(),
    fetchExchange(),
    sseExchange('/graphql', {
      credentials: 'include',
    }),
  ],
  onError: (err, event) => {
    if (err instanceof TypieError) {
      error(err.status, { message: err.message });
    }

    if (err instanceof NetworkError) {
      if (err.statusCode === 401) {
        redirect(302, event.url.href);
      }

      error(err.statusCode ?? 500, { message: err.message });
    }
  },
});
