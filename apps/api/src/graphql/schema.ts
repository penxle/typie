import './enums.ts';
import './objects.ts';
import './resolvers/admin.ts';
import './resolvers/auth.ts';
import './resolvers/blob.ts';
import './resolvers/changeset.ts';
import './resolvers/document.ts';
import './resolvers/document-comment.ts';
import './resolvers/entity.ts';
import './resolvers/folder.ts';
import './resolvers/font.ts';
import './resolvers/internal.ts';
import './resolvers/llm.ts';
import './resolvers/note.ts';
import './resolvers/stats.ts';
import './resolvers/payment.ts';
import './resolvers/post.ts';
import './resolvers/redirect.ts';
import './resolvers/search.ts';
import './resolvers/site.ts';
import './resolvers/text.ts';
import './resolvers/unfurl.ts';
import './resolvers/user.ts';
import './resolvers/widget.ts';
import './resolvers/export.ts';
import './resolvers/feedback.ts';

import { dev } from '#/env.ts';
import { builder } from './builder.ts';

export const schema = builder.toSchema();

if (dev) {
  const { writeFileSync } = await import('node:fs');
  const { lexicographicSortSchema, printSchema } = await import('graphql');
  writeFileSync('schema.graphql', `${printSchema(lexicographicSortSchema(schema))}\n`);
}
