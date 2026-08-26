export type PrismAccessCode = 'ok' | 'prism_beta_required' | 'subscription_required' | 'ai_opt_in_required' | 'prism_credit_insufficient';

export const parseAllowlist = (raw: string): string[] =>
  raw
    .split(',')
    .map((id) => id.trim())
    .filter(Boolean);

export const evaluatePrismAccess = (input: {
  allowlisted: boolean;
  entitled: boolean;
  aiOptIn: boolean;
  credit: { balance: number; required: number } | null;
}): PrismAccessCode => {
  if (!input.allowlisted) return 'prism_beta_required';
  if (!input.entitled) return 'subscription_required';
  if (!input.aiOptIn) return 'ai_opt_in_required';
  if (input.credit !== null && input.credit.balance < input.credit.required) return 'prism_credit_insufficient';
  return 'ok';
};
