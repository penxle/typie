import * as Sentry from '@sentry/node';
import { redis } from '#/cache.ts';
import * as slack from '#/external/slack.ts';

export type OpsAlertId =
  | 'already-paid-without-evidence'
  | 'already-paid-amount-mismatch'
  | 'payment-credit-shortfall'
  | 'plan-change-predecessor-ambiguous'
  | 'iap-promotion-conflict-skipped'
  | 'iap-live-contract-enroll-rejected'
  | 'iap-ownership-mismatch'
  | 'apple-transaction-id-mismatch'
  | 'apple-unbound-discovery-unresolved'
  | 'iap-succession-target-unregistered'
  | 'iap-succession-token-unknown-globally'
  | 'iap-foreign-predecessor-observed'
  | 'google-token-gone-live-canonical'
  | 'google-token-not-found'
  | 'iap-unsupported-store-payload'
  | 'iap-ingest-invalid-payload'
  | 'iap-ingest-unknown-order-state'
  | 'iap-ingest-orders-all-not-found'
  | 'google-pending-refund-review'
  | 'google-acknowledge-failed'
  | 'iap-unbound-independent-notification'
  | 'prism-credit-charge-unknown'
  | 'prism-credit-refund-incomplete'
  | 'invariant-violation';

export const opsAlert = async (id: OpsAlertId, context: Record<string, unknown>) => {
  Sentry.captureMessage(`ops:${id}`, { level: 'error', extra: context });

  await slack.sendMessage({
    channel: '#alert',
    username: '운영 알림',
    iconEmoji: ':rotating_light:',
    message: `\`\`\`\n${JSON.stringify({ id, ...context }, null, 2)}\n\`\`\``,
  });
};

const OPS_ALERT_DEDUPE_TTL_SECONDS = 86_400;

// 재시도·일일 재조정이 같은 사실을 반복 보고하면 실제 신규 발생이 그 안에 묻힌다 — id·키 당 하루 1회로 접는다.
// 레디스 실패는 알람 발화 쪽으로 폴백한다(디듀프 저장소 장애가 관측을 없애서는 안 된다).
export const opsAlertOnce = async (id: OpsAlertId, dedupeKey: string, context: Record<string, unknown>) => {
  let acquired: boolean;

  try {
    acquired = (await redis.set(`ops-alert-dedupe:${id}:${dedupeKey}`, '1', 'EX', OPS_ALERT_DEDUPE_TTL_SECONDS, 'NX')) === 'OK';
  } catch {
    acquired = true;
  }

  if (acquired) {
    await opsAlert(id, context);
  }
};
