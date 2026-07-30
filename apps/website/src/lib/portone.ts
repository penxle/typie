import * as PortOne from '@portone/browser-sdk/v2';

const STORE_ID = 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1';
const KAKAOPAY_CHANNEL_KEY = 'channel-key-a9bb57da-3277-4056-99cb-1a856c2a7a00';

export type KakaoPayBillingKeyResult =
  | { status: 'succeeded'; billingKey: string }
  | { status: 'canceled' }
  | {
      status: 'failed';
      code: string | undefined;
      message: string | undefined;
      pgCode: string | undefined;
      pgMessage: string | undefined;
    };

export const requestKakaoPayBillingKey = async (params: { userId: string }): Promise<KakaoPayBillingKeyResult> => {
  try {
    const resp = await PortOne.requestIssueBillingKey({
      storeId: STORE_ID,
      channelKey: KAKAOPAY_CHANNEL_KEY,
      billingKeyMethod: 'EASY_PAY',
      issueName: '타이피 정기결제',
      issueId: `billing-key-${crypto.randomUUID()}`,
      customer: { customerId: params.userId },
    });

    if (resp === undefined) {
      return { status: 'canceled' };
    }

    if (resp.code !== undefined) {
      return { status: 'failed', code: resp.code, message: resp.message, pgCode: resp.pgCode, pgMessage: resp.pgMessage };
    }

    // 수동 승인 채널은 빌링키 대신 이 리터럴을 돌려준다 — 서버 조회를 태우면 원인 불명 실패가 된다.
    if (resp.billingKey === 'NEEDS_CONFIRMATION') {
      return { status: 'failed', code: 'NEEDS_CONFIRMATION', message: undefined, pgCode: undefined, pgMessage: undefined };
    }

    return { status: 'succeeded', billingKey: resp.billingKey };
  } catch (err) {
    return {
      status: 'failed',
      code: 'sdk_exception',
      message: err instanceof Error ? err.message : String(err),
      pgCode: undefined,
      pgMessage: undefined,
    };
  }
};
