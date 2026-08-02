import { PaymentInvoiceState, SubscriptionState } from '@typie/lib/enums';
import { and, eq, inArray, sql } from 'drizzle-orm';
import { firstOrThrow, PaymentInvoices, Subscriptions } from '#/db/index.ts';
import type { Transaction } from '#/db/index.ts';

type RetireReservationParams = {
  userId: string;
  subscriptionId?: string;
};

// WILL_ACTIVATE 예약 전용 회수(상태를 assert — 일반 구독 삭제 API가 아니다). 인보이스가 연결돼 있으면 FK 가 restrict 라 물리
// 삭제가 실패하므로 열린 인보이스를 CANCELED 로 CAS 하고 EXPIRED tombstone 으로 전이하며, 없으면 삭제한다.
export const retireReservation = async (tx: Transaction, { userId, subscriptionId }: RetireReservationParams) => {
  const reservations = await tx
    .select({ id: Subscriptions.id })
    .from(Subscriptions)
    .where(
      and(
        eq(Subscriptions.userId, userId),
        eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE),
        subscriptionId ? eq(Subscriptions.id, subscriptionId) : undefined,
      ),
    )
    .for('no key update');

  const retired: string[] = [];
  for (const reservation of reservations) {
    const invoiceCount = await tx
      .select({ count: sql<number>`count(*)::int` })
      .from(PaymentInvoices)
      .where(eq(PaymentInvoices.subscriptionId, reservation.id))
      .then(firstOrThrow);

    if (invoiceCount.count === 0) {
      // UserTrials.subscription_id 도 restrict 참조다 — 트라이얼 예약은 존재하지 않으므로(빌링키 예약 전용) 삭제가 안전하다.
      await tx.delete(Subscriptions).where(eq(Subscriptions.id, reservation.id));
    } else {
      await tx
        .update(PaymentInvoices)
        .set({ state: PaymentInvoiceState.CANCELED })
        .where(
          and(
            eq(PaymentInvoices.subscriptionId, reservation.id),
            inArray(PaymentInvoices.state, [PaymentInvoiceState.UPCOMING, PaymentInvoiceState.OVERDUE]),
          ),
        );
      await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.id, reservation.id));
    }
    retired.push(reservation.id);
  }
  return retired;
};
