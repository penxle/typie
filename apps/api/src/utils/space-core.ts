import { SpaceState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, asc, eq, inArray } from 'drizzle-orm';
import { Spaces } from '#/db/schemas/tables.ts';
import type { Database, Transaction } from '#/db/index.ts';

export type SpaceLink = { label: string; url: string };

const isHttpUrl = (url: string): boolean => {
  try {
    const { protocol } = new URL(url);
    return protocol === 'http:' || protocol === 'https:';
  } catch {
    return false;
  }
};

export const normalizeSpaceLinks = (links: readonly SpaceLink[]): SpaceLink[] => {
  const normalized = links
    .map((link) => ({ label: link.label.trim(), url: link.url.trim() }))
    .filter((link) => link.label.length > 0 && link.url.length > 0);

  for (const link of normalized) {
    if (!isHttpUrl(link.url)) throw new TypieError({ code: 'space_link_invalid', status: 400 });
  }

  return normalized;
};

export const buildSpacesBySiteQuery = (executor: Database | Transaction, input: { siteIds: string[] }) =>
  executor
    .select()
    .from(Spaces)
    .where(and(inArray(Spaces.siteId, input.siteIds), eq(Spaces.state, SpaceState.ACTIVE)))
    .orderBy(asc(Spaces.createdAt));
