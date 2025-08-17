import { redirect } from '@sveltejs/kit';
import type { IndexPage_Query_AfterLoad } from './$graphql';

export const _IndexPage_Query_AfterLoad: IndexPage_Query_AfterLoad = async ({ query }) => {
  if (query.me) {
    redirect(302, '/initial');
  }
};
