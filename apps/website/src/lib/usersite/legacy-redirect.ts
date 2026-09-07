export const legacyEntityRedirect = (input: { protocol: string; usersiteHost: string; slug: string; search?: string }) =>
  `${input.protocol}//${input.usersiteHost}/s/${input.slug}${input.search ?? ''}`;
