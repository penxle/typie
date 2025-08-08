import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
import type { AdminPosts_Query_Variables } from './$graphql';

export const _AdminPosts_Query_Variables: AdminPosts_Query_Variables = ({ url }) => {
  const searchParams = url.searchParams;
  const page = Number(searchParams.get('page') ?? '1');
  const search = searchParams.get('search') ?? undefined;

  const offset = (page - 1) * ADMIN_ITEMS_PER_PAGE;

  return {
    search,
    offset,
    limit: ADMIN_ITEMS_PER_PAGE,
  };
};
