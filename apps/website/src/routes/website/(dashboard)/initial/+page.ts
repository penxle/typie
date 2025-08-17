import { redirect } from '@sveltejs/kit';
import type { InitialPage_Query_AfterLoad } from './$graphql';

export const _InitialPage_Query_AfterLoad: InitialPage_Query_AfterLoad = async ({ query }) => {
  if (query.me.preferences.initialPage === 'home') {
    redirect(302, '/home');
  }

  const recentEntity = query.me.recentlyViewedEntities[0];
  if (recentEntity) {
    redirect(302, `/${recentEntity.slug}`);
  }

  const firstEntity = query.me.sites[0].firstEntity;
  if (firstEntity) {
    redirect(302, `/${firstEntity.slug}`);
  }
};
