export const legacyEntityRedirect = (input: { protocol: string; usersiteHost: string; slug: string; search?: string }) =>
  `${input.protocol}//${input.usersiteHost}/s/${input.slug}${input.search ?? ''}`;

export const legacyHomeRedirect = (input: { protocol: string; usersiteHost: string; host: string; search?: string }) => {
  const suffix = `.${input.usersiteHost}`;
  if (!input.host.endsWith(suffix)) return null;
  const slug = input.host.slice(0, -suffix.length);
  if (slug.length === 0 || slug.includes('.')) return null;
  return `${input.protocol}//${input.usersiteHost}/@${slug}${input.search ?? ''}`;
};
