import { eq } from 'drizzle-orm';
import { db, firstOrThrow, Users } from '#/db/index.ts';
import { env } from '#/env.ts';
import * as linear from '#/external/linear.ts';
import { builder } from '../builder.ts';
import { formatFeedbackDescription } from './feedback-format.ts';

const labelMap: Record<string, string> = env.LINEAR_LABEL_MAP ? JSON.parse(env.LINEAR_LABEL_MAP) : {};

builder.mutationFields((t) => ({
  submitFeedback: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      topic: t.input.string({ required: false }),
      content: t.input.string(),
      mood: t.input.string({ required: false }),
      url: t.input.string({ required: false }),
      platform: t.input.string({ required: false }),
      osVersion: t.input.string({ required: false }),
      appVersion: t.input.string({ required: false }),
      deviceName: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const user = await db
        .select({ id: Users.id, name: Users.name, email: Users.email })
        .from(Users)
        .where(eq(Users.id, ctx.session.userId))
        .then(firstOrThrow);

      const trimmed = input.content.trim();
      const title = trimmed.length > 50 ? `${trimmed.slice(0, 50)}…` : trimmed;
      const description = formatFeedbackDescription({
        content: input.content,
        user,
        mood: input.mood,
        url: input.url,
        platform: input.platform,
        osVersion: input.osVersion,
        appVersion: input.appVersion,
        deviceName: input.deviceName,
      });

      const labelId = input.topic ? labelMap[input.topic] : undefined;

      await linear.createIssue({
        title,
        description,
        labelIds: labelId ? [labelId] : undefined,
      });

      return true;
    },
  }),
}));
