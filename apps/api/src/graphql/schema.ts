import './enums';
import './objects';
import './resolvers/admin';
import './resolvers/auth';
import './resolvers/blob';
import './resolvers/canvas';
import './resolvers/comment';
import './resolvers/entity';
import './resolvers/folder';
import './resolvers/font';
import './resolvers/internal';
import './resolvers/note';
import './resolvers/notification';
import './resolvers/stats';
import './resolvers/payment';
import './resolvers/post';
import './resolvers/search';
import './resolvers/site';
import './resolvers/unfurl';
import './resolvers/user';
import './resolvers/export';

import { dev } from '@/env';
import { builder } from './builder';

export const schema = builder.toSchema();

if (dev) {
  const { writeFileSync } = await import('node:fs');
  const { lexicographicSortSchema, printSchema } = await import('graphql');
  writeFileSync('schema.graphql', `${printSchema(lexicographicSortSchema(schema))}\n`);
}
