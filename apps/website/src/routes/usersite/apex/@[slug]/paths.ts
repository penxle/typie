export const spaceHomePath = (slug: string) => `/@${slug}`;
export const folderPath = (slug: string, number: string) => `${spaceHomePath(slug)}/f/${number}`;
export const tagPath = (slug: string, name: string) => `${spaceHomePath(slug)}/t/${encodeURIComponent(name)}`;
export const publicationPath = (slug: string, number: string) => `${spaceHomePath(slug)}/p/${number}`;
export const lowercaseSpaceRedirectPath = (pathname: string, slug: string) => {
  const lowered = slug.toLowerCase();
  if (lowered === slug || !/^[\da-z-]+$/.test(lowered)) return null;
  return `${spaceHomePath(lowered)}${pathname.replace(/^\/@[^/]*/, '')}`;
};
