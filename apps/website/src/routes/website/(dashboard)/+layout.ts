import { redirect } from '@sveltejs/kit';
import type { DashboardLayout_Query_AfterLoad } from './$graphql';

export const _DashboardLayout_Query_AfterLoad: DashboardLayout_Query_AfterLoad = ({ query }) => {
  if (!query.me) {
    redirect(302, '/auth/login');
  }
};

export const ssr = false;
