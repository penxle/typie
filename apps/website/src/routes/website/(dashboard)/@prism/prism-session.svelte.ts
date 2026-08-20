import { SessionStore } from '@typie/ui/state';

export const createPrismSessionState = (userId: string) => new SessionStore<string | null>(`typie:prism:session:${userId}`, null);
