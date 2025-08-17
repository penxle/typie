import { redirect } from '@sveltejs/kit';
import type { AdminLayout_Query_AfterLoad } from './$graphql';

export const _AdminLayout_Query_AfterLoad: AdminLayout_Query_AfterLoad = ({ query }) => {
  if (!query.me || query.me.role !== 'ADMIN') {
    redirect(302, '/initial');
  }
};

export const ssr = false;
