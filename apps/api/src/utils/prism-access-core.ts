export type PrismAccessCode = 'ok' | 'subscription_required' | 'ai_opt_in_required' | 'prism_credit_insufficient';

export const evaluatePrismAccess = (input: {
  entitled: boolean;
  aiOptIn: boolean;
  credit: { balance: number; required: number } | null;
}): PrismAccessCode => {
  if (!input.entitled) return 'subscription_required';
  if (!input.aiOptIn) return 'ai_opt_in_required';
  if (input.credit !== null && input.credit.balance < input.credit.required) return 'prism_credit_insufficient';
  return 'ok';
};
