export const spaceHomePath = (slug: string) => `/@${slug}`;
export const seriesListPath = (slug: string) => `${spaceHomePath(slug)}/s`;
export const seriesPath = (slug: string, collectionId: string) => `${spaceHomePath(slug)}/s/${collectionId}`;
export const tagPath = (slug: string, name: string) => `${spaceHomePath(slug)}/t/${encodeURIComponent(name)}`;
export const publicationPath = (slug: string, id: string) => `${spaceHomePath(slug)}/p/${id}`;
export const lowercaseSpaceRedirectPath = (pathname: string, slug: string) => {
  const lowered = slug.toLowerCase();
  if (lowered === slug || !/^[\da-z-]+$/.test(lowered)) return null;
  return `${spaceHomePath(lowered)}${pathname.replace(/^\/@[^/]*/, '')}`;
};
