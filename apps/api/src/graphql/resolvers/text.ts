import { TextReplacementState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, eq, isNotNull, or } from 'drizzle-orm';
import { db, firstOrThrow, TableCode, TextReplacementPreferences, TextReplacements, validateDbId } from '#/db/index.ts';
import { generateFractionalOrder } from '#/utils/index.ts';
import { builder } from '../builder.ts';
import { isTypeOf, TextReplacement, TextReplacementPreference, User } from '../objects.ts';

TextReplacement.implement({
  isTypeOf: isTypeOf(TableCode.TEXT_REPLACEMENTS),
  fields: (t) => ({
    id: t.exposeID('id'),
    match: t.exposeString('match'),
    substitute: t.exposeString('substitute'),
    regex: t.exposeBoolean('regex'),
    preset: t.exposeBoolean('preset'),
    note: t.exposeString('note', { nullable: true }),
    order: t.exposeString('order', { nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

TextReplacementPreference.implement({
  isTypeOf: isTypeOf(TableCode.TEXT_REPLACEMENT_PREFERENCES),
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: TextReplacementState }),
    order: t.exposeString('order', { nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),

    user: t.expose('userId', { type: User }),
    textReplacement: t.expose('textReplacementId', { type: TextReplacement }),
  }),
});

const TextReplacementNode = builder.unionType('TextReplacementNode', {
  types: [TextReplacement, TextReplacementPreference],
});

builder.objectFields(User, (t) => ({
  textReplacements: t.field({
    type: [TextReplacementNode],
    resolve: async (parent) => {
      const rows = await db
        .select({ textReplacement: TextReplacements, preference: TextReplacementPreferences })
        .from(TextReplacements)
        .leftJoin(
          TextReplacementPreferences,
          and(eq(TextReplacementPreferences.textReplacementId, TextReplacements.id), eq(TextReplacementPreferences.userId, parent.id)),
        )
        .where(or(eq(TextReplacements.preset, true), isNotNull(TextReplacementPreferences.id)))
        .orderBy(asc(TextReplacements.order), asc(TextReplacementPreferences.order));

      return rows.map((row) => row.preference ?? row.textReplacement);
    },
  }),
}));

builder.mutationFields((t) => ({
  createTextReplacement: t.withAuth({ session: true }).fieldWithInput({
    type: TextReplacementNode,
    input: {
      match: t.input.string(),
      substitute: t.input.string(),
      regex: t.input.boolean({ required: false }),
      note: t.input.string({ required: false }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db.transaction(async (tx) => {
        const textReplacement = await tx
          .insert(TextReplacements)
          .values({
            match: input.match,
            substitute: input.substitute,
            regex: input.regex ?? false,
            note: input.note || null,
            preset: false,
          })
          .returning()
          .then(firstOrThrow);

        return await tx
          .insert(TextReplacementPreferences)
          .values({
            userId: ctx.session.userId,
            textReplacementId: textReplacement.id,
            state: TextReplacementState.ACTIVE,
            order: generateFractionalOrder({
              lower: input.lowerOrder,
              upper: input.upperOrder,
            }),
          })
          .returning()
          .then(firstOrThrow);
      });
    },
  }),

  updateTextReplacement: t.withAuth({ session: true }).fieldWithInput({
    type: TextReplacementNode,
    input: {
      textReplacementId: t.input.id({ validate: validateDbId(TableCode.TEXT_REPLACEMENTS) }),
      match: t.input.string({ required: false }),
      substitute: t.input.string({ required: false }),
      regex: t.input.boolean({ required: false }),
      note: t.input.string({ required: false }),
      state: t.input.field({ type: TextReplacementState, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const textReplacement = await db
        .select()
        .from(TextReplacements)
        .where(eq(TextReplacements.id, input.textReplacementId))
        .then(firstOrThrow);

      if (textReplacement.preset) {
        if (
          (input.match !== null && input.match !== undefined) ||
          (input.substitute !== null && input.substitute !== undefined) ||
          (input.regex !== null && input.regex !== undefined) ||
          (input.note !== null && input.note !== undefined)
        ) {
          throw new TypieError({ code: 'preset_immutable', status: 400 });
        }

        if (input.state === TextReplacementState.DISABLED) {
          return await db
            .insert(TextReplacementPreferences)
            .values({
              userId: ctx.session.userId,
              textReplacementId: textReplacement.id,
              state: TextReplacementState.DISABLED,
            })
            .onConflictDoUpdate({
              target: [TextReplacementPreferences.userId, TextReplacementPreferences.textReplacementId],
              set: { state: TextReplacementState.DISABLED },
            })
            .returning()
            .then(firstOrThrow);
        }

        if (input.state === TextReplacementState.ACTIVE) {
          await db
            .delete(TextReplacementPreferences)
            .where(
              and(
                eq(TextReplacementPreferences.userId, ctx.session.userId),
                eq(TextReplacementPreferences.textReplacementId, textReplacement.id),
              ),
            );
        }

        return textReplacement;
      }

      const preference = await db
        .select()
        .from(TextReplacementPreferences)
        .where(
          and(
            eq(TextReplacementPreferences.textReplacementId, textReplacement.id),
            eq(TextReplacementPreferences.userId, ctx.session.userId),
          ),
        )
        .then(firstOrThrow);

      if (
        (input.match !== null && input.match !== undefined) ||
        (input.substitute !== null && input.substitute !== undefined) ||
        (input.regex !== null && input.regex !== undefined) ||
        (input.note !== null && input.note !== undefined)
      ) {
        await db
          .update(TextReplacements)
          .set({
            match: input.match ?? undefined,
            substitute: input.substitute ?? undefined,
            regex: input.regex ?? undefined,
            note: input.note !== null && input.note !== undefined ? input.note || null : undefined,
            updatedAt: dayjs(),
          })
          .where(eq(TextReplacements.id, textReplacement.id));
      }

      if (input.state !== null && input.state !== undefined) {
        return await db
          .update(TextReplacementPreferences)
          .set({ state: input.state })
          .where(eq(TextReplacementPreferences.id, preference.id))
          .returning()
          .then(firstOrThrow);
      }

      return preference;
    },
  }),

  deleteTextReplacement: t.withAuth({ session: true }).fieldWithInput({
    type: TextReplacementNode,
    input: {
      textReplacementId: t.input.id({ validate: validateDbId(TableCode.TEXT_REPLACEMENTS) }),
    },
    resolve: async (_, { input }, ctx) => {
      const textReplacement = await db
        .select()
        .from(TextReplacements)
        .where(eq(TextReplacements.id, input.textReplacementId))
        .then(firstOrThrow);

      if (textReplacement.preset) {
        throw new TypieError({ code: 'preset_not_deletable', status: 400 });
      }

      await db
        .select({ id: TextReplacementPreferences.id })
        .from(TextReplacementPreferences)
        .where(
          and(
            eq(TextReplacementPreferences.textReplacementId, textReplacement.id),
            eq(TextReplacementPreferences.userId, ctx.session.userId),
          ),
        )
        .then(firstOrThrow);

      return await db.delete(TextReplacements).where(eq(TextReplacements.id, textReplacement.id)).returning().then(firstOrThrow);
    },
  }),

  moveTextReplacement: t.withAuth({ session: true }).fieldWithInput({
    type: TextReplacementNode,
    input: {
      textReplacementId: t.input.id({ validate: validateDbId(TableCode.TEXT_REPLACEMENTS) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .update(TextReplacementPreferences)
        .set({
          order: generateFractionalOrder({
            lower: input.lowerOrder,
            upper: input.upperOrder,
          }),
        })
        .where(
          and(
            eq(TextReplacementPreferences.textReplacementId, input.textReplacementId),
            eq(TextReplacementPreferences.userId, ctx.session.userId),
          ),
        )
        .returning()
        .then(firstOrThrow);
    },
  }),
}));
