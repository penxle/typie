export type ThreadState = 'open' | 'closed' | 'resolved' | 'withdrawn';

export const canClose = (state: ThreadState): boolean => state === 'open';
export const canReopen = (state: ThreadState): boolean => state === 'closed';
