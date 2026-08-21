export const backoffDelay = (delays: readonly number[], attempt: number): number | null => delays[attempt - 1] ?? null;
