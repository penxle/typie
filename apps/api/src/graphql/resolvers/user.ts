import { builder } from '../builder';
import { User } from '../objects';

/**
 * * Types
 */

User.implement({
  grantScopes: (user, context) => (user.id === context.session?.userId ? ['$owner'] : []),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    email: t.exposeString('email'),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  me: t.field({
    type: User,
    nullable: true,
    resolve: async (_, __, ctx) => {
      return ctx.session?.userId;
    },
  }),
}));

/**
 * * Mutations
 */

// builder.mutationFields((t) => ({
// }));
