import dedent from 'dedent';
import { Hono } from 'hono';
import { pgr } from '#/db/index.ts';
import { env } from '#/env.ts';
import type { Env } from '#/context.ts';

export const bmo = new Hono<Env>();

const verifyApiSecret = (header: string | undefined) => {
  if (!header) return false;
  const token = header.startsWith('Bearer ') ? header.slice(7) : header;
  return token === env.BMO_API_KEY;
};

bmo.post('/query', async (c) => {
  if (!verifyApiSecret(c.req.header('authorization'))) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  const { query } = await c.req.json<{ query: string }>();
  if (!query) {
    return c.json({ error: 'Missing query' }, 400);
  }

  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder();
      const heartbeat = setInterval(() => controller.enqueue(encoder.encode(' ')), 1000);

      try {
        const result = await pgr.begin('READ ONLY', async (sql) => {
          const rows = await sql.unsafe(query);
          return { success: true as const, count: rows.length, rows: [...rows] };
        });
        controller.enqueue(encoder.encode(JSON.stringify(result)));
      } catch (err) {
        controller.enqueue(encoder.encode(JSON.stringify({ success: false, error: err instanceof Error ? err.message : String(err) })));
      } finally {
        clearInterval(heartbeat);
        controller.close();
      }
    },
  });

  return new Response(stream, { headers: { 'content-type': 'application/json' } });
});

let schema: unknown | null = null;

// spell-checker:disable
const getSchemaQuery = dedent`
  WITH table_info AS (
    SELECT
      t.table_name,
      obj_description(c.oid) as table_comment
    FROM information_schema.tables t
    LEFT JOIN pg_catalog.pg_class c ON c.relname = t.table_name AND c.relnamespace = (
      SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = t.table_schema
    )
    WHERE t.table_schema = 'public'
      AND t.table_type = 'BASE TABLE'
  ),
  column_info AS (
    SELECT
      c.table_name,
      c.column_name,
      c.data_type,
      c.is_nullable,
      c.column_default,
      c.ordinal_position,
      col_description(pgc.oid, c.ordinal_position) as column_comment,
      CASE
        WHEN fk.constraint_name IS NOT NULL THEN
          json_build_object(
            'table', fk.foreign_table_name,
            'column', fk.foreign_column_name
          )
        ELSE NULL
      END as foreign_key
    FROM information_schema.columns c
    LEFT JOIN pg_catalog.pg_class pgc ON pgc.relname = c.table_name AND pgc.relnamespace = (
      SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = c.table_schema
    )
    LEFT JOIN (
      SELECT
        kcu.table_name,
        kcu.column_name,
        ccu.table_name AS foreign_table_name,
        ccu.column_name AS foreign_column_name,
        tc.constraint_name
      FROM information_schema.table_constraints tc
      JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
      JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name = tc.constraint_name
      WHERE tc.constraint_type = 'FOREIGN KEY'
    ) fk ON fk.table_name = c.table_name AND fk.column_name = c.column_name
    WHERE c.table_schema = 'public'
  ),
  index_info AS (
    SELECT
      tablename as table_name,
      indexname,
      indexdef
    FROM pg_indexes
    WHERE schemaname = 'public'
  ),
  enum_info AS (
    SELECT
      t.typname as enum_name,
      array_agg(e.enumlabel ORDER BY e.enumsortorder) as enum_values
    FROM pg_type t
    JOIN pg_enum e ON t.oid = e.enumtypid
    JOIN pg_namespace n ON n.oid = t.typnamespace
    WHERE n.nspname = 'public'
      AND t.typtype = 'e'
    GROUP BY t.typname
  )
  SELECT json_build_object(
    'tables', (
      SELECT json_agg(
        json_build_object(
          'table_name', t.table_name,
          'table_comment', t.table_comment,
          'columns', (
            SELECT json_agg(
              json_build_object(
                'column_name', c.column_name,
                'data_type', c.data_type,
                'is_nullable', c.is_nullable = 'YES',
                'column_default', c.column_default,
                'column_comment', c.column_comment,
                'foreign_key', c.foreign_key
              ) ORDER BY c.ordinal_position
            )
            FROM column_info c
            WHERE c.table_name = t.table_name
          ),
          'indexes', (
            SELECT json_agg(
              json_build_object(
                'index_name', i.indexname,
                'index_def', i.indexdef
              )
            )
            FROM index_info i
            WHERE i.table_name = t.table_name
          )
        ) ORDER BY t.table_name
      )
      FROM table_info t
    ),
    'enums', (
      SELECT json_agg(
        json_build_object(
          'enum_name', e.enum_name,
          'enum_values', e.enum_values
        ) ORDER BY e.enum_name
      )
      FROM enum_info e
    )
  ) as schema;
`;
// spell-checker:enable

bmo.get('/schema', async (c) => {
  if (!verifyApiSecret(c.req.header('authorization'))) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  if (!schema) {
    await pgr.begin('READ ONLY', async (sql) => {
      const result = await sql.unsafe(getSchemaQuery);
      schema = result[0].schema;
    });
  }

  return c.json(schema);
});
