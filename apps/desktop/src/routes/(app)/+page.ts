import { redirect } from '@sveltejs/kit';
import type { App_Query_AfterLoad } from './$graphql';

export const _App_Query_AfterLoad: App_Query_AfterLoad = async ({ query }) => {
  if (!query.me) {
    redirect(302, '/auth/login');
  }
};
