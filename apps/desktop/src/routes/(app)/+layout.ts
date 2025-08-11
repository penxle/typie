import { redirect } from '@sveltejs/kit';
import type { Layout_Query_AfterLoad } from './$graphql';

export const _Layout_Query_AfterLoad: Layout_Query_AfterLoad = async ({ query }) => {
  if (!query.me) {
    redirect(302, '/auth/login');
  }
};
