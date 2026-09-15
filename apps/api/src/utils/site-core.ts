import { TypieError } from '@typie/lib/errors';

export type SiteLink = { label: string; url: string };

const isHttpUrl = (url: string): boolean => {
  try {
    const { protocol } = new URL(url);
    return protocol === 'http:' || protocol === 'https:';
  } catch {
    return false;
  }
};

export const normalizeSiteLinks = (links: readonly SiteLink[]): SiteLink[] => {
  const normalized = links
    .map((link) => ({ label: link.label.trim(), url: link.url.trim() }))
    .filter((link) => link.label.length > 0 && link.url.length > 0);

  for (const link of normalized) {
    if (!isHttpUrl(link.url)) throw new TypieError({ code: 'site_link_invalid', status: 400 });
  }

  return normalized;
};
