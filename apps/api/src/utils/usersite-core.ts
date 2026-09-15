import escape from 'escape-string-regexp';

export const parseUsersiteSlug = (origin: string, usersiteUrl: string): string | null => {
  const pattern = new RegExp(`^${escape(usersiteUrl).replace(String.raw`\*\.`, String.raw`([^.]+)\.`)}$`);
  return origin.match(pattern)?.[1] ?? null;
};

export const isUsersiteApexOrigin = (origin: string, usersiteUrl: string) => origin === usersiteApexUrl(usersiteUrl);

export const usersiteApexUrl = (usersiteUrl: string) => usersiteUrl.replace('*.', '');

export const siteUrl = (usersiteUrl: string, slug: string) => `${usersiteApexUrl(usersiteUrl)}/@${slug}`;

export const publicationUrl = (usersiteUrl: string, slug: string, number: string) => `${siteUrl(usersiteUrl, slug)}/p/${number}`;

export const folderUrl = (usersiteUrl: string, slug: string, number: string) => `${siteUrl(usersiteUrl, slug)}/f/${number}`;
