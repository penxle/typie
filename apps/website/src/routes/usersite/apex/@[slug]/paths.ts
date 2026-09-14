export const spaceHomePath = (slug: string) => `/@${slug}`;
export const seriesListPath = (slug: string) => `${spaceHomePath(slug)}/s`;
export const seriesPath = (slug: string, permalink: string) => `${spaceHomePath(slug)}/s/${permalink}`;
export const tagPath = (slug: string, name: string) => `${spaceHomePath(slug)}/t/${encodeURIComponent(name)}`;
export const publicationPath = (slug: string, permalink: string) => `${spaceHomePath(slug)}/p/${permalink}`;
export const lowercaseSpaceRedirectPath = (pathname: string, slug: string) => {
  const lowered = slug.toLowerCase();
  if (lowered === slug || !/^[\da-z-]+$/.test(lowered)) return null;
  return `${spaceHomePath(lowered)}${pathname.replace(/^\/@[^/]*/, '')}`;
};
