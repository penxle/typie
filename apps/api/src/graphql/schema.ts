import './enums';
import './objects';
import './resolvers/auth';
import './resolvers/blob';
import './resolvers/comment';
import './resolvers/entity';
import './resolvers/folder';
import './resolvers/internal';
import './resolvers/notification';
import './resolvers/payment';
import './resolvers/post';
import './resolvers/preorder';
import './resolvers/search';
import './resolvers/site';
import './resolvers/unfurl';
import './resolvers/user';

import { dev } from '@/env';
import { builder } from './builder';

export const schema = builder.toSchema();

if (dev) {
  const { writeFileSync } = await import('node:fs');
  const { lexicographicSortSchema, printSchema } = await import('graphql');
  writeFileSync('schema.graphql', `${printSchema(lexicographicSortSchema(schema))}\n`);
}
