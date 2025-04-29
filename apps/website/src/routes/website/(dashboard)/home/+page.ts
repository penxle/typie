import { redirect } from '@sveltejs/kit';
import { LocalStore } from '$lib/state';
import type { AppPreference } from '$lib/context';
import type { HomePage_Query_AfterLoad } from './$graphql';

export const _HomePage_Query_AfterLoad: HomePage_Query_AfterLoad = async ({ query }) => {
  const pref = LocalStore.get<AppPreference>('typie:pref');

  if (pref?.currentPage) {
    redirect(302, `/${pref.currentPage}`);
  }

  const entity = query.me.sites[0].firstEntity;
  if (entity) {
    redirect(302, `/${entity.slug}`);
  }
};
