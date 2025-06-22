import { ADMIN_ITEMS_PER_PAGE } from '$lib/constants';
import type { AdminUsers_Query_Variables } from './$graphql';

export const _AdminUsers_Query_Variables: AdminUsers_Query_Variables = ({ url }) => {
  const page = Number(url.searchParams.get('page')) || 1;
  const search = url.searchParams.get('search') || undefined;

  return {
    search,
    offset: (page - 1) * ADMIN_ITEMS_PER_PAGE,
    limit: ADMIN_ITEMS_PER_PAGE,
  };
};
