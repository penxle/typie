import escape from 'escape-string-regexp';

export const parseUsersiteSlug = (origin: string, usersiteUrl: string): string | null => {
  const pattern = new RegExp(`^${escape(usersiteUrl).replace(String.raw`\*\.`, String.raw`([^.]+)\.`)}$`);
  return origin.match(pattern)?.[1] ?? null;
};

export const isUsersiteApexOrigin = (origin: string, usersiteUrl: string) => origin === usersiteUrl.replace('*.', '');
