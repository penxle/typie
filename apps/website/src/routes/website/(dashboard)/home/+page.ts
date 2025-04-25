import { redirect } from '@sveltejs/kit';
import type { HomePage_Query_AfterLoad } from './$graphql';

export const _HomePage_Query_AfterLoad: HomePage_Query_AfterLoad = async ({ query }) => {
  const entity = query.me.sites[0].firstEntity;
  if (entity) {
    redirect(302, `/${entity.slug}`);
  }
};
