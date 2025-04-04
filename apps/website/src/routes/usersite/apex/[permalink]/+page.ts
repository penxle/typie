import { redirect } from '@sveltejs/kit';
import type { UsersiteApexPermalinkPage_Query_AfterLoad, UsersiteApexPermalinkPage_Query_Variables } from './$graphql';

export const _UsersiteApexPermalinkPage_Query_Variables: UsersiteApexPermalinkPage_Query_Variables = ({ params }) => ({
  permalink: params.permalink,
});

export const _UsersiteApexPermalinkPage_Query_AfterLoad: UsersiteApexPermalinkPage_Query_AfterLoad = (query) => {
  redirect(302, `${query.entityViewByPermalink.site.url}/${query.entityViewByPermalink.slug}`);
};
