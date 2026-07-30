export type BillingKeyIssuance = {
  status: string;
  customerId: string | undefined;
  channelKeys: string[];
};

export type BillingKeyVerification =
  { ok: true } | { ok: false; reason: 'not_issued' | 'customer_missing' | 'customer_mismatch' | 'channel_mismatch' };

type VerifyEasyPayBillingKeyParams = {
  userId: string;
  channelKey: string;
};

export const verifyEasyPayBillingKey = (issuance: BillingKeyIssuance, params: VerifyEasyPayBillingKeyParams): BillingKeyVerification => {
  if (issuance.status !== 'ISSUED') {
    return { ok: false, reason: 'not_issued' };
  }

  if (!issuance.customerId) {
    return { ok: false, reason: 'customer_missing' };
  }

  if (issuance.customerId !== params.userId) {
    return { ok: false, reason: 'customer_mismatch' };
  }

  if (!issuance.channelKeys.includes(params.channelKey)) {
    return { ok: false, reason: 'channel_mismatch' };
  }

  return { ok: true };
};
