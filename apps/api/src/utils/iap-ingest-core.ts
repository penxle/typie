import { InAppPurchaseRecordState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import type { JWSTransactionDecodedPayload } from '@apple/app-store-server-library';
import type { androidpublisher_v3 } from '@googleapis/androidpublisher';

export type IapPaymentRecordDraft = {
  identifier: string;
  productId: string | null;
  state: InAppPurchaseRecordState;
  amount: string;
  currency: string;
  refundedAmount: string | null;
  purchasedAt: dayjs.Dayjs;
  refundedAt: dayjs.Dayjs | null;
  data: unknown;
};

type GoogleMoney = { units?: string | null; nanos?: number | null };

const formatDecimal = (units: string, frac: number, fracDigits: number): string => {
  if (frac === 0) {
    return units;
  }

  return `${units}.${String(frac).padStart(fracDigits, '0')}`.replace(/0+$/, '');
};

export const milliunitsToDecimal = (value: number): string => {
  return formatDecimal(String(Math.trunc(value / 1000)), Math.abs(value % 1000), 3);
};

export const googleMoneyToDecimal = (money: GoogleMoney | null | undefined): string => {
  return formatDecimal(money?.units ?? '0', Math.abs(money?.nanos ?? 0), 9);
};

export const sumGoogleMoneyDecimal = (moneys: (GoogleMoney | null | undefined)[]): string => {
  let units = 0n;
  let nanos = 0n;

  for (const money of moneys) {
    units += BigInt(money?.units ?? '0');
    nanos += BigInt(money?.nanos ?? 0);
  }

  units += nanos / 1_000_000_000n;
  nanos %= 1_000_000_000n;

  return googleMoneyToDecimal({ units: String(units), nanos: Number(nanos) });
};

// 갱신 주문 ID는 "기저..N" 형태로 증가한다 — 최신 ID에서 기저를 뽑아 전 회차를 열거한다.
// 접미가 숫자가 아니거나 상한 밖이면 열거하지 않고 그 ID만 반환한다 — 서드파티 발급 값으로
// 배열 크기를 정하지 않는다(비정상 접미사의 열거는 프로세스 OOM으로 직결).
const ORDER_CHAIN_SUFFIX_LIMIT = 1000;

export const deriveGoogleOrderChain = (latestSuccessfulOrderId: string): string[] => {
  const separatorIndex = latestSuccessfulOrderId.lastIndexOf('..');
  if (separatorIndex === -1) {
    return [latestSuccessfulOrderId];
  }

  const base = latestSuccessfulOrderId.slice(0, separatorIndex);
  const suffix = Number(latestSuccessfulOrderId.slice(separatorIndex + 2));
  if (!Number.isSafeInteger(suffix) || suffix < 0 || suffix > ORDER_CHAIN_SUFFIX_LIMIT) {
    return [latestSuccessfulOrderId];
  }

  return [base, ...Array.from({ length: suffix + 1 }, (_, index) => `${base}..${index}`)];
};

export type AppleTransactionMapping =
  | { kind: 'record'; record: IapPaymentRecordDraft; appAccountToken: string | null }
  | { kind: 'skip' }
  | { kind: 'invalid'; reason: string };

export const mapAppleTransaction = (
  transaction: JWSTransactionDecodedPayload,
  { allowSandbox }: { allowSandbox: boolean },
): AppleTransactionMapping => {
  if (transaction.environment !== 'Production' && !(allowSandbox && transaction.environment === 'Sandbox')) {
    return { kind: 'skip' };
  }

  // FAMILY_SHARED 는 이 계정의 결제가 아니다 — 구매자 계정의 이력에서 PURCHASED 로 적재된다.
  if (transaction.inAppOwnershipType !== 'PURCHASED') {
    return { kind: 'skip' };
  }

  if (!transaction.transactionId || transaction.purchaseDate === undefined) {
    return { kind: 'invalid', reason: 'missing-identity' };
  }

  // price 0(무료 체험 등)은 유효한 결제 사건이다 — undefined 만 결손이다.
  if (transaction.price === undefined || transaction.currency === undefined) {
    return { kind: 'invalid', reason: 'missing-amount' };
  }

  const amount = milliunitsToDecimal(transaction.price);
  const refunded = transaction.revocationDate !== undefined;

  return {
    kind: 'record',
    record: {
      identifier: transaction.transactionId,
      productId: transaction.productId ?? null,
      state: refunded ? InAppPurchaseRecordState.REFUNDED : InAppPurchaseRecordState.PAID,
      amount,
      currency: transaction.currency,
      // Apple 은 부분 환불액을 노출하지 않는다 — 환불은 전액으로 처리한다.
      refundedAmount: refunded ? amount : null,
      purchasedAt: dayjs(transaction.purchaseDate),
      refundedAt: refunded ? dayjs(transaction.revocationDate) : null,
      data: transaction,
    },
    appAccountToken: transaction.appAccountToken ?? null,
  };
};

const GOOGLE_PAID_ORDER_STATES = new Set(['PROCESSED', 'PENDING_REFUND']);
const GOOGLE_REFUNDED_ORDER_STATES = new Set(['REFUNDED', 'PARTIALLY_REFUNDED']);
const GOOGLE_UNSETTLED_ORDER_STATES = new Set(['PENDING', 'CANCELED']);

export type GoogleOrderMapping =
  | { kind: 'record'; record: IapPaymentRecordDraft }
  | { kind: 'skip' }
  | { kind: 'invalid'; reason: string }
  | { kind: 'unknown-state'; state: string };

export const mapGoogleOrder = (order: androidpublisher_v3.Schema$Order): GoogleOrderMapping => {
  const state = order.state ?? 'undefined';

  if (GOOGLE_UNSETTLED_ORDER_STATES.has(state)) {
    return { kind: 'skip' };
  }

  if (!GOOGLE_PAID_ORDER_STATES.has(state) && !GOOGLE_REFUNDED_ORDER_STATES.has(state)) {
    return { kind: 'unknown-state', state };
  }

  if (!order.orderId || !order.createTime) {
    return { kind: 'invalid', reason: 'missing-identity' };
  }

  if (!order.total?.currencyCode) {
    return { kind: 'invalid', reason: 'missing-amount' };
  }

  const amount = googleMoneyToDecimal(order.total);
  const refunded = GOOGLE_REFUNDED_ORDER_STATES.has(state);

  let refundedAmount: string | null = null;
  let refundedAt: dayjs.Dayjs | null = null;

  if (refunded) {
    const refundEvent = order.orderHistory?.refundEvent;
    const partialRefundEvents = order.orderHistory?.partialRefundEvents ?? [];

    if (state === 'PARTIALLY_REFUNDED') {
      const totals = partialRefundEvents.map((event) => event.refundDetails?.total);
      if (totals.length === 0 || totals.some((total) => !total)) {
        return { kind: 'invalid', reason: 'missing-refund-detail' };
      }

      refundedAmount = sumGoogleMoneyDecimal(totals);
    } else {
      refundedAmount = refundEvent?.refundDetails?.total ? googleMoneyToDecimal(refundEvent.refundDetails.total) : amount;
    }

    // 스토어가 환불 시각을 안 주면 null 로 남긴다 — 합성 시각(now)은 무조건 갱신하는 upsert 와 겹치면
    // 재수집마다 앞으로 밀려 수렴이 깨진다.
    const lastPartialAt = partialRefundEvents.at(-1)?.processTime ?? partialRefundEvents.at(-1)?.createTime;
    const refundedAtSource = refundEvent?.eventTime ?? lastPartialAt;
    refundedAt = refundedAtSource ? dayjs(refundedAtSource) : null;
  }

  return {
    kind: 'record',
    record: {
      identifier: order.orderId,
      productId: order.lineItems?.[0]?.productId ?? null,
      state: refunded ? InAppPurchaseRecordState.REFUNDED : InAppPurchaseRecordState.PAID,
      amount,
      currency: order.total.currencyCode,
      refundedAmount,
      purchasedAt: dayjs(order.createTime),
      refundedAt,
      data: order,
    },
  };
};
