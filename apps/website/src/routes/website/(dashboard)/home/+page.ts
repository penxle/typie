import { redirect } from '@sveltejs/kit';
import { LocalStore } from '$lib/state';
import type { HomePage_Query_AfterLoad } from './$graphql';

export const _HomePage_Query_AfterLoad: HomePage_Query_AfterLoad = async ({ query }) => {
  const lvp = LocalStore.get<Record<string, string>>('typie:lvp');

  const slug = lvp?.[query.me.sites[0].id];
  if (slug) {
    redirect(302, `/${slug}`);
  }

  const entity = query.me.sites[0].firstEntity;
  if (entity) {
    redirect(302, `/${entity.slug}`);
  }
};
