import { getClientAddress } from '@typie/lib';
import DataLoader from 'dataloader';
import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import stringify from 'fast-json-stable-stringify';
import { setCookie } from 'hono/cookie';
import { nanoid } from 'nanoid';
import * as R from 'remeda';
import { db, first, UserAccessTokens } from '@/db';
import type { Context as HonoContext } from 'hono';

type LoaderParams<Key, Result, SortKey, Nullability extends boolean, Many extends boolean> = {
  name: string;
  nullable?: Nullability;
  many?: Many;
  key: (value: Nullability extends true ? Result | null : Result) => Nullability extends true ? SortKey | null : SortKey;
  load: (keys: Key[]) => Promise<Result[]>;
};

export type ServerContext = HonoContext<Env>;

type DefaultContext = {
  ip: string;
  deviceId: string;

  loader: <
    Key = string,
    Result = unknown,
    SortKey = Key,
    Nullability extends boolean = false,
    Many extends boolean = false,
    MaybeResult = Nullability extends true ? Result | null : Result,
    FinalResult = Many extends true ? MaybeResult[] : MaybeResult,
  >(
    params: LoaderParams<Key, Result, SortKey, Nullability, Many>,
  ) => DataLoader<Key, FinalResult, string>;
  ' $loaders': Map<string, DataLoader<unknown, unknown>>;
};

export type SessionContext = {
  session: {
    sessionId: string;
    tokenId: string;
    userId: string;
  };
};

export type Context = DefaultContext & Partial<SessionContext>;

export type UserContext = Context & {
  c: ServerContext;
};

export type Env = {
  Variables: { context: Context };
};

export const deriveContext = async (c: ServerContext): Promise<Context> => {
  let deviceId = c.req.header('X-Device-Id');
  if (!deviceId) {
    deviceId = nanoid(32);
    setCookie(c, 'typie-did', deviceId, {
      path: '/',
      httpOnly: true,
      secure: true,
      sameSite: 'lax',
      maxAge: dayjs.duration(1, 'year').asSeconds(),
    });
  }

  const ctx: Context = {
    ip: getClientAddress(c),
    deviceId,
    loader: ({ name, nullable, many, load, key }) => {
      const cached = ctx[' $loaders'].get(name);
      if (cached) {
        return cached as never;
      }

      const loader = new DataLoader(
        async (keys) => {
          const rows = await load(keys as never);
          const values = R.groupBy(rows, (row) => stringify(key(row)));
          return keys.map((key) => {
            const value = values[stringify(key)];

            if (value?.length) {
              return many ? value : value[0];
            }

            if (nullable) {
              return null;
            }

            if (many) {
              return [];
            }

            return new Error(`DataLoader(${name}): Missing key`);
          });
        },
        { cache: false },
      );

      ctx[' $loaders'].set(name, loader);

      return loader as never;
    },
    ' $loaders': new Map(),
  };

  const authorization = c.req.header('Authorization');
  const accessToken = authorization?.match(/^Bearer\s+(.+)$/)?.[1];
  if (accessToken) {
    const token = await db
      .select({
        id: UserAccessTokens.id,
        userId: UserAccessTokens.userId,
        sessionId: UserAccessTokens.sessionId,
      })
      .from(UserAccessTokens)
      .where(eq(UserAccessTokens.token, accessToken))
      .then(first);

    if (token) {
      ctx.session = {
        sessionId: token.sessionId,
        tokenId: token.id,
        userId: token.userId,
      };
    }
  }

  return ctx;
};
