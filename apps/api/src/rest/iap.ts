import { DeliveryStatus, RefundPreference } from '@apple/app-store-server-library';
import * as Sentry from '@sentry/node';
import { InAppPurchaseStore, PlanAvailability, SubscriptionState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { and, desc, eq, inArray, lt, lte, ne, sql } from 'drizzle-orm';
import { Hono } from 'hono';
import { match } from 'ts-pattern';
import { db, first, Plans, Subscriptions, UserInAppPurchases, UserTrials } from '#/db/index.ts';
import { production } from '#/env.ts';
import * as appstore from '#/external/appstore.ts';
import * as googleplay from '#/external/googleplay.ts';
import * as slack from '#/external/slack.ts';
import { lockUserSubscriptionState } from '#/utils/subscription-lock.ts';
import type { ResponseBodyV2 } from '@apple/app-store-server-library';
import type { Env } from '#/context.ts';
import type { DeveloperNotification } from '#/external/googleplay.ts';

export const iap = new Hono<Env>();

const getLiveInAppPurchaseSubscription = async (userId: string) => {
  return await db
    .select({
      id: Subscriptions.id,
      state: Subscriptions.state,
      expiresAt: Subscriptions.expiresAt,
    })
    .from(Subscriptions)
    .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
    .where(
      and(
        eq(Subscriptions.userId, userId),
        eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE),
        inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
      ),
    )
    .orderBy(desc(eq(Subscriptions.state, SubscriptionState.ACTIVE)), desc(Subscriptions.createdAt))
    .then(first);
};

iap.post('/appstore', async (c) => {
  const body = await c.req.json<ResponseBodyV2>();
  if (!body.signedPayload) {
    return c.json({ error: 'invalid_request' }, 400);
  }

  const notification = await appstore.decodeNotification(body.signedPayload);
  const originalTransactionId = notification.data.transaction?.originalTransactionId;
  const planId = notification.data.transaction?.productId?.toUpperCase();

  if (!originalTransactionId) {
    await slack.sendMessage({
      channel: 'iap',
      username: '인앱결제 알림',
      iconEmoji: ':credit_card:',
      message: `\`\`\`\n${JSON.stringify({ source: 'rest/appstore', reason: 'no_transaction', notification }, null, 2)}\n\`\`\``,
    });
    return c.json({}, 200);
  }

  const inAppPurchase = await db
    .select({
      userId: UserInAppPurchases.userId,
    })
    .from(UserInAppPurchases)
    .where(and(eq(UserInAppPurchases.identifier, originalTransactionId), eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE)))
    .then(first);

  if (!inAppPurchase) {
    return c.json({}, 200);
  }

  const subscription = await getLiveInAppPurchaseSubscription(inAppPurchase.userId);

  await match(notification.notificationType)
    .with('DID_RENEW', 'SUBSCRIBED', 'OFFER_REDEEMED', 'REFUND_REVERSED', async () => {
      const purchaseDate = dayjs(notification.data.transaction?.purchaseDate);
      const expiresDate = dayjs(notification.data.transaction?.expiresDate);
      const plan = planId ? await db.select({ id: Plans.id }).from(Plans).where(eq(Plans.id, planId)).then(first) : null;

      if (subscription) {
        // 스냅샷 비교 후 무조건 갱신하면 동시·역순 재전송이 최신 갱신을 짧은 만료일로 되돌릴 수 있다.
        // 조건을 UPDATE 안으로 옮겨(커밋된 최신 행 값과 재평가됨) 만료일 단조 증가를 원자적으로 보장한다.
        // 동시 환불로 EXPIRED 가 된 행은 되살리지 않는다 — 웹훅 UPDATE 는 EXPIRED 를 벗어나게 하지 않는다(부활은 insert·재조정 경로만).
        await db
          .update(Subscriptions)
          .set({
            state: SubscriptionState.ACTIVE,
            ...(plan && { planId: plan.id }),
            renewedAt: purchaseDate,
            expiresAt: expiresDate,
          })
          .where(
            and(
              eq(Subscriptions.id, subscription.id),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              lte(Subscriptions.expiresAt, expiresDate),
            ),
          );
      } else if (plan && expiresDate.isAfter(dayjs())) {
        // 알림의 서명 데이터는 발행 시점 기준이라(최대 72시간 재전송) 환불 이후 도착한 stale 갱신이
        // 새 ACTIVE 행을 만들 수 있다. 삽입 전에 현재 스토어 상태로 자격을 확인해 이 부활 창을 닫는다.
        // 조회~삽입 사이에 환불이 일어나는 경우는 이후 도착하는 REFUND/REVOKE 웹훅이 이 행을 회수한다.
        const status = await appstore.getSubscriptionStatus(originalTransactionId);
        if (status.kind === 'error' || status.kind === 'unknown') {
          // 자격을 판정할 수 없으면 삽입도 폐기도 하지 않고 5xx 로 재전송을 유도한다 — 신규 결제를 조용히 잃지 않는다.
          throw new Error('appstore subscription status lookup failed');
        }
        if (status.kind !== 'active' && status.kind !== 'grace') {
          return;
        }

        await db.transaction(async (tx) => {
          await lockUserSubscriptionState(tx, inAppPurchase.userId);

          // 락 대기 중 바인딩이 삭제·이전(재등록/탈퇴 — 탈퇴는 바인딩을 지운다)됐으면 stale userId 로 만들지 않는다.
          const freshBinding = await tx
            .select({ userId: UserInAppPurchases.userId })
            .from(UserInAppPurchases)
            .where(
              and(
                eq(UserInAppPurchases.userId, inAppPurchase.userId),
                eq(UserInAppPurchases.store, InAppPurchaseStore.APP_STORE),
                eq(UserInAppPurchases.identifier, originalTransactionId),
              ),
            )
            .then(first);

          if (!freshBinding) {
            return;
          }

          const upserted = await tx
            .insert(Subscriptions)
            .values({
              userId: inAppPurchase.userId,
              planId: plan.id,
              startsAt: purchaseDate,
              expiresAt: expiresDate,
              renewedAt: purchaseDate,
              state: SubscriptionState.ACTIVE,
            })
            .onConflictDoUpdate({
              target: [Subscriptions.userId],
              targetWhere: eq(Subscriptions.state, SubscriptionState.ACTIVE),
              set: { planId: plan.id, startsAt: purchaseDate, expiresAt: expiresDate, renewedAt: purchaseDate },
              // 충돌하는 ACTIVE 행이 IAP 이고 만료일이 역행하지 않을 때만 갱신한다.
              // 다른 채널(빌링키 등)의 ACTIVE 구독을 덮어쓰지 않고, stale 재전송이 최신 갱신을 되돌리지 못하게 한다.
              setWhere: and(
                inArray(
                  Subscriptions.planId,
                  tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)),
                ),
                lte(Subscriptions.expiresAt, expiresDate),
              ),
            })
            .returning({ id: Subscriptions.id });

          if (upserted.length === 0) {
            // 스토어는 이미 과금했는데 비IAP ACTIVE 와 충돌해 조용히 사라지는 구매 — 운영이 인지해야 수동 환불이 가능하다.
            Sentry.captureMessage('iap webhook upsert skipped by conflicting non-iap active subscription', {
              level: 'warning',
              extra: { userId: inAppPurchase.userId, store: 'APP_STORE', identifier: originalTransactionId },
            });
          }
        });
      }
    })
    .with('EXPIRED', 'GRACE_PERIOD_EXPIRED', async () => {
      if (!subscription) {
        return;
      }

      // Apple 은 실패한 알림을 최대 72시간 재전송하고, DID_RENEW 유실 시 로컬 만료일이 갱신을 반영하지 못한다.
      // 로컬 만료일만으론 stale 여부를 구분할 수 없으므로 현재 스토어 상태를 직접 조회해 실제 비활성일 때만 회수한다.
      const status = await appstore.getSubscriptionStatus(originalTransactionId);
      if (status.kind === 'error') {
        // 스토어 조회 실패 — 회수를 보류하고 5xx 로 재전송을 유도한다.
        throw new Error('appstore subscription status lookup failed');
      }

      if (status.kind === 'active') {
        // 스토어는 여전히 활성 = stale EXPIRED. 갱신이 반영되지 않았으면 만료일을 끌어올려 락아웃을 막는다.
        // renewedAt 은 스냅샷이 아닌 갱신 시점의 실제 이전 만료일(행의 현재 값)을 쓴다.
        // 스토어 조회~갱신 사이 동시 REFUND/REVOKE 가 EXPIRED+만료일 clip 을 먼저 커밋하면 만료일 조건만으론
        // 환불된 행을 부활시키므로, EXPIRED 가 아닌 행만 복구한다.
        if (status.expiresDate) {
          const storeExpiresAt = dayjs(status.expiresDate);
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.ACTIVE, renewedAt: sql`${Subscriptions.expiresAt}`, expiresAt: storeExpiresAt })
            .where(
              and(
                eq(Subscriptions.id, subscription.id),
                ne(Subscriptions.state, SubscriptionState.EXPIRED),
                lt(Subscriptions.expiresAt, storeExpiresAt),
              ),
            );
        }
        return;
      }

      if (status.kind === 'grace') {
        // 유예 중 — 회수하지 않는다.
        return;
      }

      if (status.kind === 'suspended') {
        // 재청구 중 — 권한은 없지만(만료일 경과로 이미 차단) 복구 가능하므로 종료하지 않고 live 로 남겨
        // 재조정 크론이 계속 다루게 한다. EXPIRED 로 보내면 재조정 대상에서 빠져 복구 경로가 사라진다.
        await db
          .update(Subscriptions)
          .set({ state: SubscriptionState.WILL_EXPIRE })
          .where(
            and(
              eq(Subscriptions.id, subscription.id),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              lte(Subscriptions.expiresAt, dayjs()),
            ),
          );
        return;
      }

      if (status.kind === 'unknown') {
        // 조회는 됐으나 이 트랜잭션을 확인할 수 없음 — 판정을 보류한다(재조정 크론이 다시 다룬다).
        return;
      }

      // 확정 종료(expired/revoked). expired 는 조회~갱신 사이 동시 갱신이 만료일을 미래로 올렸으면 건드리지
      // 않도록 CAS(만료일이 현재 이하일 때만)로 회수하고, revoked(환불·철회)는 스토어 확정 종료이므로 즉시 회수한다.
      await db
        .update(Subscriptions)
        .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
        .where(
          status.kind === 'revoked'
            ? eq(Subscriptions.id, subscription.id)
            : and(eq(Subscriptions.id, subscription.id), lte(Subscriptions.expiresAt, dayjs())),
        );
    })
    .with('DID_CHANGE_RENEWAL_PREF', async () => {
      if (subscription && planId) {
        const plan = await db.select({ id: Plans.id }).from(Plans).where(eq(Plans.id, planId)).then(first);
        if (plan) {
          // stale 재전송이 갱신된 만료일을 되돌리지 못하게 단조 가드를 건다. 갱신이 사이에 끼었다면
          // 그 갱신 트랜잭션이 이미 최신 플랜을 반영하므로(DID_RENEW 가 planId 동기화) 여기서 건너뛰어도 안전하다.
          const expiresDate = dayjs(notification.data.transaction?.expiresDate);
          await db
            .update(Subscriptions)
            .set({ planId, expiresAt: expiresDate })
            .where(
              and(
                eq(Subscriptions.id, subscription.id),
                ne(Subscriptions.state, SubscriptionState.EXPIRED),
                lte(Subscriptions.expiresAt, expiresDate),
              ),
            );
        }
      }
    })
    .with('DID_CHANGE_RENEWAL_STATUS', async () => {
      if (subscription) {
        // 상태 전제조건을 스냅샷이 아닌 UPDATE 조건으로 검사해 동시 전이(환불로 EXPIRED 등)와 원자적으로 배타한다.
        if (notification.subtype === 'AUTO_RENEW_DISABLED') {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.WILL_EXPIRE })
            .where(and(eq(Subscriptions.id, subscription.id), eq(Subscriptions.state, SubscriptionState.ACTIVE)));
        } else if (notification.subtype === 'AUTO_RENEW_ENABLED') {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.ACTIVE })
            .where(and(eq(Subscriptions.id, subscription.id), eq(Subscriptions.state, SubscriptionState.WILL_EXPIRE)));
        }
      }
    })
    .with('RENEWAL_EXTENDED', async () => {
      if (subscription) {
        // 연장은 만료일이 늘어나는 방향만 유효하다 — stale 재전송이 갱신된 만료일을 되돌리지 못하게 한다.
        const expiresDate = dayjs(notification.data.transaction?.expiresDate);
        await db
          .update(Subscriptions)
          .set({ expiresAt: expiresDate })
          .where(
            and(
              eq(Subscriptions.id, subscription.id),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              lte(Subscriptions.expiresAt, expiresDate),
            ),
          );
      }
    })
    .with('REFUND', 'REVOKE', async () => {
      if (subscription) {
        await db
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.id, subscription.id));
      }
    })
    .with('CONSUMPTION_REQUEST', async () => {
      const transactionId = notification.data.transaction?.transactionId;
      if (!transactionId) {
        return;
      }

      const trial = await db.select({ id: UserTrials.id }).from(UserTrials).where(eq(UserTrials.userId, inAppPurchase.userId)).then(first);

      await appstore.sendConsumptionInformation(transactionId, {
        customerConsented: true,
        sampleContentProvided: !!trial,
        deliveryStatus: DeliveryStatus.DELIVERED,
        refundPreference: RefundPreference.DECLINE,
      });
    })
    .with('DID_FAIL_TO_RENEW', async () => {
      if (subscription) {
        // 알림의 서명된 만료일은 갱신에 실패한 기간의 끝이다. 로컬 만료일이 그보다 뒤면 이미 더 최신 갱신
        // (DID_RENEW)이 반영된 것이므로, 최대 72시간 재전송되는 stale 실패 알림이 새 기간을 회수하지 못하게 한다.
        const failedExpiresDate = dayjs(notification.data.transaction?.expiresDate);
        await db
          .update(Subscriptions)
          .set(
            notification.subtype === 'GRACE_PERIOD'
              ? { state: SubscriptionState.IN_GRACE_PERIOD }
              : // 유예 없이 재청구(billing retry): 권한은 없지만 복구 가능하므로 WILL_EXPIRE(만료일 경과)로 둔다.
                { state: SubscriptionState.WILL_EXPIRE, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` },
          )
          .where(
            and(
              eq(Subscriptions.id, subscription.id),
              ne(Subscriptions.state, SubscriptionState.EXPIRED),
              lte(Subscriptions.expiresAt, failedExpiresDate),
            ),
          );
      }
    })
    .otherwise(async () => {
      await slack.sendMessage({
        channel: 'iap',
        username: '인앱결제 알림',
        iconEmoji: ':credit_card:',
        message: `\`\`\`\n${JSON.stringify({ source: 'rest/appstore', notification }, null, 2)}\n\`\`\``,
      });
    });

  return c.json({}, 200);
});

iap.post('/googleplay', async (c) => {
  const notification = await c.req.json<DeveloperNotification>();

  if (notification.subscriptionNotification) {
    const purchaseToken = notification.subscriptionNotification.purchaseToken;

    // 조회 실패(일시적 오류 포함)는 throw 하여 5xx 로 응답 → Pub/Sub 가 재전송한다. 200 으로 삼키면 알림이 영구 유실된다.
    const googlePlaySubscription = await googleplay.getSubscription(purchaseToken);

    const item = googlePlaySubscription.lineItems?.[0];
    const planId = item?.offerDetails?.basePlanId?.toUpperCase();

    if (!item || !planId) {
      return c.json({ error: 'invalid_request' }, 400);
    }

    const inAppPurchase = await db
      .select({
        userId: UserInAppPurchases.userId,
      })
      .from(UserInAppPurchases)
      .where(
        and(
          eq(UserInAppPurchases.identifier, notification.subscriptionNotification.purchaseToken),
          eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY),
        ),
      )
      .then(first);

    // 구글 플레이는 발생한 알림이 환경 상관 없이 prod/dev 모두 발송됨
    if (!inAppPurchase) {
      if (production) {
        // prod 환경에서는 inAppPurchase 없을 시 오류 반환하고 pubsub에 재시도 맡김
        return c.json({ error: 'invalid_request' }, 400);
      }
      // dev 환경에서는 inAppPurchase 없어도 무시함
      return c.json({}, 200);
    }

    const subscription = await getLiveInAppPurchaseSubscription(inAppPurchase.userId);

    await match(googlePlaySubscription.subscriptionState)
      .with('SUBSCRIPTION_STATE_ACTIVE', async () => {
        const expiresAt = dayjs(item.expiryTime);
        if (subscription) {
          // 스냅샷 비교가 아닌 조건부 UPDATE 로 만료일 단조 증가를 원자적으로 보장한다(동시·역순 전달 대비).
          // renewedAt 은 스냅샷이 아닌 갱신 시점의 실제 이전 만료일(행의 현재 값)을 쓴다.
          // 동시 환불로 EXPIRED 가 된 행은 되살리지 않는다 — 웹훅 UPDATE 는 EXPIRED 를 벗어나게 하지 않는다(부활은 insert·재조정 경로만).
          const renewed = await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.ACTIVE, planId, renewedAt: sql`${Subscriptions.expiresAt}`, expiresAt })
            .where(
              and(
                eq(Subscriptions.id, subscription.id),
                ne(Subscriptions.state, SubscriptionState.EXPIRED),
                lt(Subscriptions.expiresAt, expiresAt),
              ),
            )
            .returning({ id: Subscriptions.id });

          if (renewed.length === 0) {
            // 만료일이 같은 재전송·상태 복구(재구독 등) — 상태·플랜만 동기화한다. 만료일이 이미 앞서 있으면 no-op.
            await db
              .update(Subscriptions)
              .set({ state: SubscriptionState.ACTIVE, planId })
              .where(
                and(
                  eq(Subscriptions.id, subscription.id),
                  ne(Subscriptions.state, SubscriptionState.EXPIRED),
                  eq(Subscriptions.expiresAt, expiresAt),
                ),
              );
          }
        } else if (expiresAt.isAfter(dayjs())) {
          const startsAt = dayjs(googlePlaySubscription.startTime);
          await db.transaction(async (tx) => {
            await lockUserSubscriptionState(tx, inAppPurchase.userId);

            // 락 대기 중 바인딩이 삭제·이전(재등록/탈퇴 — 탈퇴는 바인딩을 지운다)됐으면 stale userId 로 만들지 않는다.
            const freshBinding = await tx
              .select({ userId: UserInAppPurchases.userId })
              .from(UserInAppPurchases)
              .where(
                and(
                  eq(UserInAppPurchases.userId, inAppPurchase.userId),
                  eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY),
                  eq(UserInAppPurchases.identifier, purchaseToken),
                ),
              )
              .then(first);

            if (!freshBinding) {
              return;
            }

            const upserted = await tx
              .insert(Subscriptions)
              .values({
                userId: inAppPurchase.userId,
                planId,
                startsAt,
                expiresAt,
                renewedAt: startsAt,
                state: SubscriptionState.ACTIVE,
              })
              .onConflictDoUpdate({
                target: [Subscriptions.userId],
                targetWhere: eq(Subscriptions.state, SubscriptionState.ACTIVE),
                set: { planId, startsAt, expiresAt, renewedAt: startsAt },
                // 충돌하는 ACTIVE 행이 IAP 이고 만료일이 역행하지 않을 때만 갱신한다.
                // 다른 채널(빌링키 등)의 ACTIVE 구독을 덮어쓰지 않고, stale 재전송이 최신 갱신을 되돌리지 못하게 한다.
                setWhere: and(
                  inArray(
                    Subscriptions.planId,
                    tx.select({ id: Plans.id }).from(Plans).where(eq(Plans.availability, PlanAvailability.IN_APP_PURCHASE)),
                  ),
                  lte(Subscriptions.expiresAt, expiresAt),
                ),
              })
              .returning({ id: Subscriptions.id });

            if (upserted.length === 0) {
              // 스토어는 이미 과금했는데 비IAP ACTIVE 와 충돌해 조용히 사라지는 구매 — 운영이 인지해야 수동 환불이 가능하다.
              Sentry.captureMessage('iap webhook upsert skipped by conflicting non-iap active subscription', {
                level: 'warning',
                extra: {
                  userId: inAppPurchase.userId,
                  store: 'GOOGLE_PLAY',
                  identifier: purchaseToken,
                },
              });
            }
          });
        }
      })
      .with('SUBSCRIPTION_STATE_EXPIRED', async () => {
        if (subscription) {
          // SUBSCRIPTION_REVOKED(12)는 스토어가 확정한 중도 환불·철회이므로 만료일이 미래여도 즉시 회수한다.
          // 자연 만료는 조회~갱신 사이 동시 갱신이 만료일을 미래로 올렸으면 건드리지 않는다(CAS).
          const revoked = notification.subscriptionNotification?.notificationType === 12;
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
            .where(
              revoked
                ? eq(Subscriptions.id, subscription.id)
                : and(eq(Subscriptions.id, subscription.id), lte(Subscriptions.expiresAt, dayjs())),
            );
        }
      })
      .with('SUBSCRIPTION_STATE_CANCELED', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.WILL_EXPIRE })
            .where(and(eq(Subscriptions.id, subscription.id), ne(Subscriptions.state, SubscriptionState.EXPIRED)));
        }
      })
      .with('SUBSCRIPTION_STATE_IN_GRACE_PERIOD', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.IN_GRACE_PERIOD })
            .where(and(eq(Subscriptions.id, subscription.id), ne(Subscriptions.state, SubscriptionState.EXPIRED)));
        }
      })
      .with('SUBSCRIPTION_STATE_ON_HOLD', async () => {
        if (subscription) {
          // 계정 보류(payment on hold): 권한은 없지만 복구 가능하므로 WILL_EXPIRE(만료일 경과)로 둔다.
          // 조회~갱신 사이 동시 갱신이 만료일을 미래로 올렸으면 건드리지 않는다(CAS).
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.WILL_EXPIRE, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
            .where(
              and(
                eq(Subscriptions.id, subscription.id),
                ne(Subscriptions.state, SubscriptionState.EXPIRED),
                lte(Subscriptions.expiresAt, dayjs()),
              ),
            );
        }
      })
      .with('SUBSCRIPTION_STATE_PAUSED', async () => {
        if (subscription) {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.WILL_EXPIRE })
            .where(and(eq(Subscriptions.id, subscription.id), ne(Subscriptions.state, SubscriptionState.EXPIRED)));
        }
      })
      .with('SUBSCRIPTION_STATE_PENDING', 'SUBSCRIPTION_STATE_PENDING_PURCHASE_CANCELED', async () => {
        // 결제 대기 중 또는 대기 중 취소 — 구독 미생성 상태이므로 처리 불필요
      })
      .otherwise(async () => {
        await slack.sendMessage({
          channel: 'iap',
          username: '인앱결제 알림',
          iconEmoji: ':credit_card:',
          message: `\`\`\`\n${JSON.stringify({ source: 'rest/googleplay', subscription }, null, 2)}\n\`\`\``,
        });
      });
  } else if (notification.voidedPurchaseNotification && notification.voidedPurchaseNotification.productType === 1) {
    const purchaseToken = notification.voidedPurchaseNotification.purchaseToken;

    const inAppPurchase = await db
      .select({ userId: UserInAppPurchases.userId })
      .from(UserInAppPurchases)
      .where(and(eq(UserInAppPurchases.identifier, purchaseToken), eq(UserInAppPurchases.store, InAppPurchaseStore.GOOGLE_PLAY)))
      .then(first);

    if (inAppPurchase) {
      // 위조 방지: 엔드포인트 인증이 없으므로, 스토어가 실제로 종료(EXPIRED)를 확인해 준 경우에만 회수한다.
      // CANCELED(만료 전까지 권한 유지)·IN_GRACE_PERIOD 등은 여전히 권한이 있으므로 "비-ACTIVE"를 회수 근거로 삼지 않는다.
      // 조회 실패(일시적 오류)는 판정하지 않고 재시도한다 — 실결제 사용자 오만료 방지.
      let revoked: boolean;
      try {
        const googlePlaySubscription = await googleplay.getSubscription(purchaseToken);
        revoked = googlePlaySubscription.subscriptionState === 'SUBSCRIPTION_STATE_EXPIRED';
      } catch (err) {
        Sentry.captureException(err);
        return c.json({ error: 'retry' }, 500);
      }

      if (revoked) {
        const subscription = await getLiveInAppPurchaseSubscription(inAppPurchase.userId);

        if (subscription) {
          await db
            .update(Subscriptions)
            .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
            .where(eq(Subscriptions.id, subscription.id));
        }
      }
    }
  } else {
    await slack.sendMessage({
      channel: 'iap',
      username: '인앱결제 알림',
      iconEmoji: ':credit_card:',
      message: `\`\`\`\n${JSON.stringify({ source: 'rest/googleplay', notification }, null, 2)}\n\`\`\``,
    });
  }

  return c.json({}, 200);
});
