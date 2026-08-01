export type AdminSearchIntent =
  { kind: 'id'; tableCode: string; id: string } | { kind: 'email'; email: string } | { kind: 'text'; text: string };

const ID_PATTERN = /^([A-Z]{1,4})0[A-Z0-9]+$/;

export const parseAdminSearchQuery = (query: string): AdminSearchIntent | null => {
  const trimmed = query.trim();

  if (trimmed.length === 0) {
    return null;
  }

  const matched = ID_PATTERN.exec(trimmed);
  if (matched) {
    return { kind: 'id', tableCode: matched[1], id: trimmed };
  }

  if (trimmed.includes('@')) {
    return { kind: 'email', email: trimmed };
  }

  return { kind: 'text', text: trimmed };
};

export type AdminSearchResultRef =
  | { __typename: 'User'; id: string }
  | { __typename: 'Entity'; id: string }
  | { __typename: 'PaymentInvoice'; user: { id: string } }
  | { __typename: 'Subscription_'; user: { id: string } }
  | { __typename: 'Site'; user: { id: string } };

export const adminSearchResultHref = (result: AdminSearchResultRef): string => {
  switch (result.__typename) {
    case 'User': {
      return `/admin/users/${result.id}`;
    }
    case 'Entity': {
      return `/admin/entities/${result.id}`;
    }
    case 'PaymentInvoice':
    case 'Subscription_': {
      return `/admin/users/${result.user.id}?tab=billing`;
    }
    case 'Site': {
      return `/admin/users/${result.user.id}?tab=contents`;
    }
  }
};
