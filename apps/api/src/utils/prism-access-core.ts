export type PrismAccessCode = 'ok' | 'prism_beta_required' | 'subscription_required' | 'ai_opt_in_required';

export const parseAllowlist = (raw: string): string[] =>
  raw
    .split(',')
    .map((id) => id.trim())
    .filter(Boolean);

export const evaluatePrismAccess = (input: { allowlisted: boolean; entitled: boolean; aiOptIn: boolean }): PrismAccessCode => {
  if (!input.allowlisted) return 'prism_beta_required';
  if (!input.entitled) return 'subscription_required';
  if (!input.aiOptIn) return 'ai_opt_in_required';
  return 'ok';
};
