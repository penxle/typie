import { getClientAddress } from '@typie/lib';
import DataLoader from 'dataloader';
import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import stringify from 'fast-json-stable-stringify';
import { getCookie, setCookie } from 'hono/cookie';
import { nanoid } from 'nanoid';
import * as R from 'remeda';
import { env } from '@/env';
import { db, first, UserSessions } from './db';
import { decodeAccessToken } from './utils';
import type { Context as HonoContext } from 'hono';

type LoaderParams<T, R, S, N extends boolean, M extends boolean> = {
  name: string;
  nullable?: N;
  many?: M;
  key: (value: N extends true ? R | null : R) => N extends true ? S | null : S;
  load: (keys: T[]) => Promise<R[]>;
};

export type ServerContext = HonoContext<Env>;

type DefaultContext = {
  ip: string;
  deviceId: string;

  loader: <T, R, S, N extends boolean = false, M extends boolean = false, RR = N extends true ? R | null : R>(
    params: LoaderParams<T, R, S, N, M>,
  ) => DataLoader<T, M extends true ? RR[] : RR, string>;
  ' $loaders': Map<string, DataLoader<unknown, unknown>>;
};

export type SessionContext = {
  session: {
    id: string;
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
  let deviceId = getCookie(c, 'typie-did');
  if (!deviceId) {
    deviceId = nanoid(32);
    setCookie(c, 'typie-did', deviceId, {
      path: '/',
      domain: env.COOKIE_DOMAIN,
      httpOnly: true,
      secure: true,
      sameSite: 'none',
      maxAge: dayjs.duration(1, 'year').asSeconds(),
    });
  }

  const ctx: Context = {
    ip: getClientAddress(c),
    deviceId,
    loader: <
      T,
      R,
      S,
      N extends boolean = false,
      M extends boolean = false,
      RR = N extends true ? R | null : R,
      F = M extends true ? RR[] : RR,
    >({
      name,
      nullable,
      many,
      load,
      key,
    }: LoaderParams<T, R, S, N, M>) => {
      const cached = ctx[' $loaders'].get(name);
      if (cached) {
        return cached as DataLoader<T, F, string>;
      }

      const loader = new DataLoader<T, F, string>(
        async (keys) => {
          const rows = await load(keys as T[]);
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
          }) as (F | Error)[];
        },
        { cache: false },
      );

      ctx[' $loaders'].set(name, loader);

      return loader;
    },
    ' $loaders': new Map(),
  };

  const accessToken = getCookie(c, 'typie-at');
  if (accessToken) {
    const sessionId = await decodeAccessToken(accessToken);
    if (sessionId) {
      const session = await db
        .select({ id: UserSessions.id, userId: UserSessions.userId })
        .from(UserSessions)
        .where(eq(UserSessions.id, sessionId))
        .then(first);

      if (session) {
        ctx.session = {
          id: session.id,
          userId: session.userId,
        };
      }
    }
  }

  return ctx;
};
