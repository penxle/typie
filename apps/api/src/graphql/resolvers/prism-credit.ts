import { PrismCreditEntryKind } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { desc, eq } from 'drizzle-orm';
import { db, PrismCreditEntries, TableCode, validateDbId } from '#/db/index.ts';
import { pubsub } from '#/pubsub.ts';
import { assertAdminPermission } from '#/utils/permission.ts';
import { adjustPrismCredit, grantPrismCredit, readPrismCreditBalance } from '#/utils/prism-credit.ts';
import { toDisplayCredits, toMilli } from '#/utils/prism-credit-core.ts';
import { builder } from '../builder.ts';
import { isTypeOf, PrismCreditEntry, User } from '../objects.ts';

const ENTRY_LIMIT_MAX = 200;

const PrismCreditBalance = builder.simpleObject('PrismCreditBalance', {
  fields: (t) => ({
    balance: t.int(),
  }),
});

const AdminPrismCreditBalance = builder.simpleObject('AdminPrismCreditBalance', {
  fields: (t) => ({
    total: t.field({ type: 'BigInt' }),
    paid: t.field({ type: 'BigInt' }),
    free: t.field({ type: 'BigInt' }),
    display: t.int(),
  }),
});

const AdminPrismCreditEntry = builder.objectRef<typeof PrismCreditEntries.$inferSelect>('AdminPrismCreditEntry').implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    kind: t.expose('kind', { type: PrismCreditEntryKind }),
    paidDelta: t.field({ type: 'BigInt', resolve: (self) => String(self.paidDelta) }),
    freeDelta: t.field({ type: 'BigInt', resolve: (self) => String(self.freeDelta) }),
    key: t.exposeString('key', { nullable: true }),
    note: t.exposeString('note', { nullable: true }),
    actor: t.field({ type: User, nullable: true, resolve: (self) => self.actorId }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

const clampLimit = (limit: number) => Math.min(Math.max(limit, 1), ENTRY_LIMIT_MAX);

const listEntries = async (userId: string, limit: number, offset: number) => {
  return await db
    .select()
    .from(PrismCreditEntries)
    .where(eq(PrismCreditEntries.userId, userId))
    .orderBy(desc(PrismCreditEntries.createdAt), desc(PrismCreditEntries.id))
    .limit(clampLimit(limit))
    .offset(Math.max(offset, 0));
};

PrismCreditEntry.implement({
  isTypeOf: isTypeOf(TableCode.PRISM_CREDIT_ENTRIES),
  fields: (t) => ({
    id: t.exposeID('id'),
    kind: t.expose('kind', { type: PrismCreditEntryKind }),
    amount: t.int({ resolve: (self) => toDisplayCredits(self.paidDelta + self.freeDelta) }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

const assertSelf = (self: { id: string }, ctx: { session?: { userId: string } }) => {
  if (ctx.session?.userId !== self.id) {
    throw new TypieError({ code: 'permission_denied' });
  }
};

builder.objectFields(User, (t) => ({
  prismCredit: t.field({
    type: PrismCreditBalance,
    resolve: async (self, _, ctx) => {
      assertSelf(self, ctx);
      const balance = await readPrismCreditBalance(db, self.id);
      return { balance: toDisplayCredits(balance.total) };
    },
  }),

  prismCreditEntries: t.field({
    type: [PrismCreditEntry],
    args: {
      limit: t.arg.int({ defaultValue: 50 }),
      offset: t.arg.int({ defaultValue: 0 }),
    },
    resolve: async (self, args, ctx) => {
      assertSelf(self, ctx);
      return await listEntries(self.id, args.limit, args.offset);
    },
  }),
}));

builder.queryFields((t) => ({
  adminPrismCredit: t.withAuth({ session: true }).field({
    type: AdminPrismCreditBalance,
    args: { userId: t.arg.string({ validate: validateDbId(TableCode.USERS) }) },
    resolve: async (_, { userId }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });
      const balance = await readPrismCreditBalance(db, userId);
      return {
        total: String(balance.total),
        paid: String(balance.paid),
        free: String(balance.free),
        display: toDisplayCredits(balance.total),
      };
    },
  }),

  adminPrismCreditEntries: t.withAuth({ session: true }).field({
    type: [AdminPrismCreditEntry],
    args: {
      userId: t.arg.string({ validate: validateDbId(TableCode.USERS) }),
      limit: t.arg.int({ defaultValue: 50 }),
      offset: t.arg.int({ defaultValue: 0 }),
    },
    resolve: async (_, args, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });
      return await listEntries(args.userId, args.limit, args.offset);
    },
  }),
}));

builder.mutationFields((t) => ({
  adminGrantPrismCredit: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      userId: t.input.string({ validate: validateDbId(TableCode.USERS) }),
      amount: t.input.int(),
      note: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      if (input.note.trim().length === 0) {
        throw new TypieError({ code: 'invalid_input', status: 400 });
      }

      if (input.amount <= 0) {
        throw new TypieError({ code: 'invalid_amount', status: 400 });
      }

      await db.transaction(async (tx) => {
        await grantPrismCredit(tx, {
          userId: input.userId,
          kind: 'GRANT',
          amount: toMilli(input.amount),
          note: input.note.trim(),
          actorId: ctx.session.userId,
        });
      });

      pubsub.publish('prism:credit', input.userId, {});

      return true;
    },
  }),

  adminAdjustPrismCredit: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      userId: t.input.string({ validate: validateDbId(TableCode.USERS) }),
      paidDelta: t.input.int(),
      freeDelta: t.input.int(),
      note: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertAdminPermission({ sessionId: ctx.session.id });

      if (input.note.trim().length === 0) {
        throw new TypieError({ code: 'invalid_input', status: 400 });
      }

      if (input.paidDelta === 0 && input.freeDelta === 0) {
        throw new TypieError({ code: 'invalid_amount', status: 400 });
      }

      await db.transaction(async (tx) => {
        await adjustPrismCredit(tx, {
          userId: input.userId,
          paidDelta: toMilli(input.paidDelta),
          freeDelta: toMilli(input.freeDelta),
          note: input.note.trim(),
          actorId: ctx.session.userId,
        });
      });

      pubsub.publish('prism:credit', input.userId, {});

      return true;
    },
  }),
}));
