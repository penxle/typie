import dedent from 'dedent';
import { eq } from 'drizzle-orm';
import { db, firstOrThrow, Users } from '#/db/index.ts';
import { env } from '#/env.ts';
import * as linear from '#/external/linear.ts';
import { builder } from '../builder.ts';

const moodLabels: Record<string, string> = {
  angry: '😠 불만',
  annoyed: '😟 아쉬움',
  good: '🙂 만족',
  great: '😄 매우 만족',
};

const labelMap: Record<string, string> = env.LINEAR_LABEL_MAP ? JSON.parse(env.LINEAR_LABEL_MAP) : {};

builder.mutationFields((t) => ({
  submitFeedback: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      topic: t.input.string({ required: false }),
      content: t.input.string(),
      mood: t.input.string({ required: false }),
      url: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const user = await db
        .select({ id: Users.id, name: Users.name, email: Users.email })
        .from(Users)
        .where(eq(Users.id, ctx.session.userId))
        .then(firstOrThrow);

      const trimmed = input.content.trim();
      const title = trimmed.length > 50 ? `${trimmed.slice(0, 50)}…` : trimmed;
      const description = dedent`
        ${input.content}

        ---

        - **사용자:** ${user.name} (${user.email})
        - **사용자 ID:** ${user.id}
        - **기분:** ${(input.mood && moodLabels[input.mood]) ?? '(없음)'}
        - **페이지:** ${input.url ?? '(없음)'}
      `;

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
