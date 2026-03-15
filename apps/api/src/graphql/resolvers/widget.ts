import { and, asc, eq } from 'drizzle-orm';
import { db, firstOrThrow, TableCode, validateDbId, Widgets } from '#/db/index.ts';
import { generateFractionalOrder } from '#/utils/index.ts';
import { builder } from '../builder.ts';
import { isTypeOf, User, Widget } from '../objects.ts';

Widget.implement({
  isTypeOf: isTypeOf(TableCode.WIDGETS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    data: t.expose('data', { type: 'JSON' }),
    order: t.exposeString('order'),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),

    user: t.expose('userId', { type: User }),
  }),
});

builder.queryFields((t) => ({
  widgets: t.withAuth({ session: true }).field({
    type: [Widget],
    resolve: async (_, __, ctx) => {
      return await db.select().from(Widgets).where(eq(Widgets.userId, ctx.session.userId)).orderBy(asc(Widgets.order));
    },
  }),
}));

builder.mutationFields((t) => ({
  createWidget: t.withAuth({ session: true }).fieldWithInput({
    type: Widget,
    input: {
      name: t.input.string(),
      data: t.input.field({ type: 'JSON' }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .insert(Widgets)
        .values({
          userId: ctx.session.userId,
          name: input.name,
          data: input.data,
          order: generateFractionalOrder({
            lower: input.lowerOrder,
            upper: input.upperOrder,
          }),
        })
        .returning()
        .then(firstOrThrow);
    },
  }),

  updateWidget: t.withAuth({ session: true }).fieldWithInput({
    type: Widget,
    input: {
      widgetId: t.input.id({ validate: validateDbId(TableCode.WIDGETS) }),
      data: t.input.field({ type: 'JSON' }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .update(Widgets)
        .set({
          data: input.data,
        })
        .where(and(eq(Widgets.id, input.widgetId), eq(Widgets.userId, ctx.session.userId)))
        .returning()
        .then(firstOrThrow);
    },
  }),

  moveWidget: t.withAuth({ session: true }).fieldWithInput({
    type: Widget,
    input: {
      widgetId: t.input.id({ validate: validateDbId(TableCode.WIDGETS) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .update(Widgets)
        .set({
          order: generateFractionalOrder({
            lower: input.lowerOrder,
            upper: input.upperOrder,
          }),
        })
        .where(and(eq(Widgets.id, input.widgetId), eq(Widgets.userId, ctx.session.userId)))
        .returning()
        .then(firstOrThrow);
    },
  }),

  deleteWidget: t.withAuth({ session: true }).fieldWithInput({
    type: Widget,
    input: {
      widgetId: t.input.id({ validate: validateDbId(TableCode.WIDGETS) }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .delete(Widgets)
        .where(and(eq(Widgets.id, input.widgetId), eq(Widgets.userId, ctx.session.userId)))
        .returning()
        .then(firstOrThrow);
    },
  }),
}));
