import { defineConfig } from 'drizzle-kit';

export default defineConfig({
  strict: true,
  verbose: true,

  schema: './src/db/schemas/*',
  out: './drizzle',

  dialect: 'postgresql',
  dbCredentials: {
    url: `${process.env.DATABASE_URL}?sslmode=no-verify`,
  },

  tablesFilter: ['!pg_stat_statements', '!pg_stat_statements_info'],

  breakpoints: false,
});
