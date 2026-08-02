package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant

class EntitlementCacheTest {
  private val now = Instant.fromEpochMilliseconds(1_700_000_000_000)

  private fun subscription(
    state: SubscriptionState = SubscriptionState.ACTIVE,
    currentPeriodEndsAt: Instant = now + 30.days,
    availability: PlanAvailability = PlanAvailability.IN_APP_PURCHASE,
  ) =
    Subscription(
      id = "SUB1",
      state = state,
      startsAt = now - 1.hours,
      currentPeriodEndsAt = currentPeriodEndsAt,
      planId = "pl0fl1map",
      planName = "FULL ACCESS",
      fee = 2900,
      availability = availability,
    )

  private fun cache(
    userId: String = "U1",
    entitled: Boolean = true,
    entitledUntil: Instant? = null,
    subscription: Subscription? = subscription(),
    checkedAt: Instant = now,
  ) =
    entitlementCacheOf(
      userId = userId,
      entitled = entitled,
      entitledUntil = entitledUntil,
      subscription = subscription,
      checkedAt = checkedAt,
    )

  @Test
  fun revocationIsStoredSoPastTrueDoesNotRevive() {
    val granted = cache(entitled = true)
    assertIs<Entitlement.Active>(resolveEntitlement(granted, now + 1.hours))

    val revoked = cache(entitled = false, subscription = null, checkedAt = now + 1.hours)
    assertFalse(revoked.entitled)
    assertEquals(Entitlement.Expired, resolveEntitlement(revoked, now + 2.hours))
  }

  @Test
  fun cacheIsBoundToSession() {
    // 계정 전환은 세션 토큰이 실제로 바뀐다. 앞선 토큰이 없는데 캐시만 남은 상태(기기 백업 복원)도 폐기다.
    assertTrue(shouldDiscardEntitlementCache("token-a", "token-b"))
    assertTrue(shouldDiscardEntitlementCache(null, "token-b"))

    // 폐기를 지나쳐 남은 타 유저 캐시도 판정에서 무시된다 — 없는 것과 같아 Unknown 이다.
    val other = cache(userId = "U1", entitled = true)
    assertIs<Entitlement.Active>(resolveEntitlement(entitlementCacheFor(other, "U1"), now))
    assertEquals(Entitlement.Unknown, resolveEntitlement(entitlementCacheFor(other, "U2"), now))
  }

  @Test
  fun sameTokenRenewKeepsCache() {
    // 매 시작마다 도는 bootstrap renew() 는 폐기 사유가 아니다.
    assertFalse(shouldDiscardEntitlementCache("token-a", "token-a"))

    val restored = cache(entitled = true, checkedAt = now - 1.days)
    assertIs<Entitlement.Active>(resolveEntitlement(restored, now))
  }

  @Test
  fun validWindowIsMinOfEntitledUntilAndMaxAge() {
    assertEquals(now + ENTITLEMENT_CACHE_MAX_AGE, entitlementCacheDeadline(cache()))
    assertEquals(now + 1.days, entitlementCacheDeadline(cache(entitledUntil = now + 1.days)))
    assertEquals(
      now + ENTITLEMENT_CACHE_MAX_AGE,
      entitlementCacheDeadline(cache(entitledUntil = now + 10.days)),
    )
  }

  @Test
  fun reconfirmingIdenticalDataExtendsTheWindow() {
    val first = cache(entitled = true, checkedAt = now)
    val reconfirmed = cache(entitled = true, checkedAt = now + 2.days)

    // 값은 그대로고 확인 시각만 다르다 — checkedAt 은 "값이 바뀐 시각"이 아니라 "서버에 확인한 시각"이다.
    assertEquals(first.entitled, reconfirmed.entitled)
    assertEquals(first.subscription, reconfirmed.subscription)

    val atFirstDeadline = now + ENTITLEMENT_CACHE_MAX_AGE
    assertEquals(Entitlement.Expired, resolveEntitlement(first, atFirstDeadline))
    assertIs<Entitlement.Active>(resolveEntitlement(reconfirmed, atFirstDeadline))
    assertEquals(
      now + 2.days + ENTITLEMENT_CACHE_MAX_AGE,
      entitlementCacheDeadline(reconfirmed),
    )
  }

  @Test
  fun outsideValidWindowIsExpiredAndAlwaysHasADeadline() {
    val cache = cache(entitled = true, entitledUntil = null)

    assertIs<Entitlement.Active>(
      resolveEntitlement(cache, now + ENTITLEMENT_CACHE_MAX_AGE - 1.hours)
    )
    assertEquals(Entitlement.Expired, resolveEntitlement(cache, now + ENTITLEMENT_CACHE_MAX_AGE))

    // entitledUntil 이 없어도 재평가 타이머가 걸릴 마감이 존재해야 한다.
    assertTrue(entitlementCacheDeadline(cache) < Instant.DISTANT_FUTURE)
  }

  @Test
  fun unknownIsOnlyForTheFirstMomentWithoutAnyValue() {
    assertEquals(Entitlement.Unknown, resolveEntitlement(null, now))

    assertIs<Entitlement.Active>(resolveEntitlement(cache(entitled = true), now))
    assertEquals(
      Entitlement.Expired,
      resolveEntitlement(cache(entitled = false, subscription = null), now),
    )
    assertEquals(Entitlement.Expired, resolveEntitlement(entitled = false, subscription = null))
  }

  @Test
  fun activeIsBuiltFromCachedSnapshot() {
    val subscription =
      subscription(state = SubscriptionState.IN_GRACE_PERIOD, availability = PlanAvailability.TRIAL)

    val entitlement =
      resolveEntitlement(cache(entitled = true, subscription = subscription), now + 1.hours)

    assertIs<Entitlement.Active>(entitlement)
    assertEquals(subscription, entitlement.subscription)
    assertTrue(entitlement.inGracePeriod)
  }

  @Test
  fun judgmentUsesEntitledNotTheSnapshot() {
    val elapsed = subscription(currentPeriodEndsAt = now - 10.days)
    assertIs<Entitlement.Active>(
      resolveEntitlement(cache(entitled = true, subscription = elapsed), now)
    )

    val alive = subscription(currentPeriodEndsAt = now + 30.days)
    assertEquals(
      Entitlement.Expired,
      resolveEntitlement(cache(entitled = false, subscription = alive), now),
    )
  }

  @Test
  fun refetchResultSupersedesAValidCache() {
    val cached = cache(entitled = true, checkedAt = now)
    assertIs<Entitlement.Active>(resolveEntitlement(cached, now + 1.hours))

    // foreground 복귀 재조회는 유효 캐시여도 수행한다 — 그 응답이 회수를 알리면 창이 남아 있어도 강등된다.
    val refetched = cache(entitled = false, subscription = null, checkedAt = now + 1.hours)
    assertEquals(Entitlement.Expired, resolveEntitlement(refetched, now + 1.hours))
    assertEquals(Entitlement.Expired, resolveEntitlement(entitled = false, subscription = null))
  }
}
