import { generateActivityImage, generateRandomName } from '@/utils';
import { builder } from '../builder';

/**
 * * Queries
 */

builder.queryFields((t) => ({
  seed: t.field({
    type: 'Float',
    resolve: () => {
      return Math.random();
    },
  }),

  randomName: t.field({
    type: 'String',
    resolve: () => {
      return generateRandomName();
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  generateRandomName: t.field({
    type: 'String',
    resolve: () => {
      return generateRandomName();
    },
  }),

  generateActivityImage: t.withAuth({ session: true }).field({
    type: 'Binary',
    resolve: async (_, __, ctx) => {
      return await generateActivityImage(ctx.session.userId);
    },
  }),
}));
