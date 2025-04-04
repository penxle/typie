import type { UsersiteWildcardSlugPage_Query_Variables } from './$graphql';

export const _UsersiteWildcardSlugPage_Query_Variables: UsersiteWildcardSlugPage_Query_Variables = ({ params, url }) => ({
  hostname: url.hostname,
  slug: params.slug,
});
