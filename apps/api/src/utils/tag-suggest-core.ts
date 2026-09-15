import { DISCOVERY_QUERY_MAX_LENGTH } from './discovery-search-core.ts';
import { decompose } from './text.ts';

export type TagSuggestion = { name: string; count: number };
export type TagSuggestions = { mine: TagSuggestion[]; popular: TagSuggestion[] };

export const TAG_SUGGESTION_LIMIT = 5;
export const TAG_SUGGEST_PREFIX_MAX_LENGTH = 20;
export const TAG_SUGGEST_GLOBAL_CURRENT_TTL_SECONDS = 300;
export const TAG_SUGGEST_GLOBAL_DATA_TTL_SECONDS = 360;
export const TAG_SUGGEST_SITE_CURRENT_TTL_SECONDS = 86_400;
export const TAG_SUGGEST_SITE_DATA_TTL_SECONDS = 86_460;

export const normalizeTag = (raw: string): string => raw.normalize('NFC').trim().replace(/^#+/, '').trim();

const searchKey = (raw: string): string => {
  const normalized = normalizeTag(raw).toLowerCase();
  return (decompose(normalized) ?? '').replaceAll(/\s+/g, ' ');
};

export const queryKey = (raw: string): string =>
  searchKey(raw.slice(0, DISCOVERY_QUERY_MAX_LENGTH)).slice(0, TAG_SUGGEST_PREFIX_MAX_LENGTH);

export const prefixesOf = (name: string): string[] => {
  const key = searchKey(name);
  const prefixes = new Set<string>(['']);
  const add = (token: string) => {
    const limit = Math.min(token.length, TAG_SUGGEST_PREFIX_MAX_LENGTH);
    for (let length = 1; length <= limit; length += 1) {
      prefixes.add(token.slice(0, length));
    }
  };
  add(key);
  for (const token of key.split(' ').slice(1)) {
    if (token.length > 0) add(token);
  }
  return [...prefixes];
};

export const globalCurrentKey = (): string => 'tag-suggest:global:current';
export const globalDataKey = (buildId: string, prefix: string): string => `tag-suggest:global:${buildId}:${prefix}`;
export const siteCurrentKey = (siteId: string): string => `tag-suggest:site:${siteId}:current`;
export const siteDataKey = (siteId: string, buildId: string, prefix: string): string => `tag-suggest:site:${siteId}:${buildId}:${prefix}`;

export const readCount = (excludeLength: number): number => TAG_SUGGESTION_LIMIT * 2 + excludeLength;

export const mergeSuggestions = (input: { mine: TagSuggestion[]; popular: TagSuggestion[]; exclude: string[] }): TagSuggestions => {
  const excluded = new Set(input.exclude.map(normalizeTag));
  const mine = input.mine.filter((suggestion) => !excluded.has(normalizeTag(suggestion.name))).slice(0, TAG_SUGGESTION_LIMIT);
  const taken = new Set([...excluded, ...mine.map((suggestion) => normalizeTag(suggestion.name))]);
  const popular = input.popular.filter((suggestion) => !taken.has(normalizeTag(suggestion.name))).slice(0, TAG_SUGGESTION_LIMIT);
  return { mine, popular };
};
