import { redirect } from '@sveltejs/kit';
import type { HomePage_Query_AfterLoad } from './$graphql';

export const _HomePage_Query_AfterLoad: HomePage_Query_AfterLoad = (query) => {
  if (!query.me) {
    redirect(302, '/auth/login');
  }
};
