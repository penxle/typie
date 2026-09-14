export const resolveLinkShareRedirect = (input: {
  requestedSlug: string;
  entitySlug: string;
  publicationUrl: string | null;
}): string | null => {
  if (input.entitySlug !== input.requestedSlug) return `/s/${input.entitySlug}`;
  return input.publicationUrl;
};
