import type { SingleSignOnProvider } from '@/enums';

export type ExternalUser = {
  provider: SingleSignOnProvider;
  principal: string;
  email: string;
  name: string | null;
  avatarUrl: string | null;
};
