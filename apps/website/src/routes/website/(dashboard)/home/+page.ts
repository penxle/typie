import { redirect } from '@sveltejs/kit';
import type { HomePage_Query_AfterLoad } from './$graphql';

export const _HomePage_Query_AfterLoad: HomePage_Query_AfterLoad = async ({ query }) => {
  const recentEntity = query.me.recentlyViewedEntities[0];
  if (recentEntity) {
    redirect(302, `/${recentEntity.slug}`);
  }

  const firstEntity = query.me.sites[0].firstEntity;
  if (firstEntity) {
    redirect(302, `/${firstEntity.slug}`);
  }
};
