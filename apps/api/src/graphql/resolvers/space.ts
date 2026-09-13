import { PublicationState, SpaceDateDisplay } from '@typie/lib/enums';
import { NotFoundError } from '@typie/lib/errors';
import { siteSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { db, TableCode, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { pubsub } from '#/pubsub.ts';
import { buildCollectionsBySpaceQuery, buildPinnedPublicationsQuery } from '#/utils/collection-core.ts';
import { enqueueDiscoverySpaceSync } from '#/utils/discovery-index.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { buildPublicationsBySpaceQuery } from '#/utils/publication-core.ts';
import { createSpaceCore, deleteSpaceCore, findActiveSpace, updateSpaceCore } from '#/utils/space.ts';
import { spaceUrl } from '#/utils/usersite-core.ts';
import { builder } from '../builder.ts';
import { Collection, Image, ISpace, isTypeOf, Publication, Site, Space } from '../objects.ts';

const SpaceLink = builder.simpleObject('SpaceLink', {
  fields: (t) => ({ label: t.string(), url: t.string() }),
});

const SpaceLinkInput = builder.inputType('SpaceLinkInput', {
  fields: (t) => ({ label: t.string(), url: t.string() }),
});

ISpace.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),
    description: t.exposeString('description', { nullable: true }),
    links: t.field({ type: [SpaceLink], resolve: (self) => self.links }),
    allowIndexing: t.exposeBoolean('allowIndexing'),
    allowDiscovery: t.exposeBoolean('allowDiscovery'),
    dateDisplay: t.expose('dateDisplay', { type: SpaceDateDisplay }),
    logo: t.field({ type: Image, nullable: true, resolve: (self) => self.logoId }),
    url: t.string({ resolve: (self) => spaceUrl(env.USERSITE_URL, self.slug) }),
  }),
});

Space.implement({
  isTypeOf: isTypeOf(TableCode.SPACES),
  interfaces: [ISpace],
  fields: (t) => ({
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    site: t.expose('siteId', { type: Site }),
    collections: t.field({
      type: [Collection],
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

        const loader = ctx.loader({
          name: 'Space.collections',
          many: true,
          load: async (ids: string[]) => await buildCollectionsBySpaceQuery(db, { spaceIds: ids }),
          key: ({ spaceId }: { spaceId: string }) => spaceId,
        });
        return await loader.load(self.id);
      },
    }),
    publications: t.field({
      type: [Publication],
      args: { states: t.arg({ type: [PublicationState], required: false }) },
      resolve: async (self, args, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

        return await buildPublicationsBySpaceQuery(db, {
          spaceIds: [self.id],
          states: args.states ?? [PublicationState.SCHEDULED, PublicationState.PUBLISHED, PublicationState.UNPUBLISHED],
        });
      },
    }),
    pinnedPublications: t.field({
      type: [Publication],
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

        const loader = ctx.loader({
          name: 'Space.pinnedPublications',
          many: true,
          load: async (ids: string[]) => await buildPinnedPublicationsQuery(db, { spaceIds: ids }),
          key: ({ spaceId }: { spaceId: string }) => spaceId,
        });
        return await loader.load(self.id);
      },
    }),
  }),
});

builder.queryFields((t) => ({
  space: t.withAuth({ session: true }).field({
    type: Space,
    args: { spaceId: t.arg.id({ validate: validateDbId(TableCode.SPACES) }) },
    resolve: async (_, args, ctx) => {
      const space = await findActiveSpace(db, args.spaceId);
      if (!space) throw new NotFoundError();
      await assertSitePermission({ userId: ctx.session.userId, siteId: space.siteId });
      return space;
    },
  }),
}));

builder.mutationFields((t) => ({
  createSpace: t.withAuth({ session: true }).fieldWithInput({
    type: Space,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      name: t.input.string(),
      slug: t.input.string({ validate: { schema: siteSchema.slug } }),
      logoId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const space = await createSpaceCore(db, { userId: ctx.session.userId, ...input });
      pubsub.publish('site:update', space.siteId, { scope: 'site' });
      await enqueueDiscoverySpaceSync([space.id]);
      return space;
    },
  }),

  updateSpace: t.withAuth({ session: true }).fieldWithInput({
    type: Space,
    input: {
      spaceId: t.input.id({ validate: validateDbId(TableCode.SPACES) }),
      name: t.input.string({ required: false }),
      slug: t.input.string({ required: false, validate: { schema: siteSchema.slug } }),
      logoId: t.input.id({ required: false, validate: validateDbId(TableCode.IMAGES) }),
      description: t.input.string({ required: false }),
      links: t.input.field({ type: [SpaceLinkInput], required: false }),
      allowIndexing: t.input.boolean({ required: false }),
      allowDiscovery: t.input.boolean({ required: false }),
      dateDisplay: t.input.field({ type: SpaceDateDisplay, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const space = await updateSpaceCore(db, { userId: ctx.session.userId, ...input });
      pubsub.publish('site:update', space.siteId, { scope: 'site' });
      if (
        input.name !== undefined ||
        input.description !== undefined ||
        input.allowIndexing !== undefined ||
        input.allowDiscovery !== undefined
      ) {
        await enqueueDiscoverySpaceSync([space.id]);
      }
      return space;
    },
  }),

  deleteSpace: t.withAuth({ session: true }).fieldWithInput({
    type: Space,
    input: { spaceId: t.input.id({ validate: validateDbId(TableCode.SPACES) }) },
    resolve: async (_, { input }, ctx) => {
      const space = await deleteSpaceCore(db, { userId: ctx.session.userId, spaceId: input.spaceId, now: dayjs() });
      pubsub.publish('site:update', space.siteId, { scope: 'site' });
      await enqueueDiscoverySpaceSync([space.id]);
      return space;
    },
  }),
}));
