import { redirect } from '@sveltejs/kit';
import type { UsersiteWildcardSlugPage_Query_AfterLoad, UsersiteWildcardSlugPage_Query_Variables } from './$graphql';

export const _UsersiteWildcardSlugPage_Query_Variables: UsersiteWildcardSlugPage_Query_Variables = ({ params, url }) => ({
  origin: url.origin,
  slug: params.slug,
});

export const _UsersiteWildcardSlugPage_Query_AfterLoad: UsersiteWildcardSlugPage_Query_AfterLoad = ({ query }) => {
  if (query.entityView.node.__typename === 'PostView' && query.entityView.node.document) {
    redirect(302, `/${query.entityView.node.document.entity.slug}`);
  }
};
