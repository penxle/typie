package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.time.Duration.Companion.days
import kotlin.time.Instant

// 유예 상한. refresh 실패 시 lastKnown 폴백(오프라인)에서 유예 상태가 무기한 유지되는 것만 막고, 온라인에서는 서버가 권위 있게 판정한다.
// 채널별 정책이 달라 상한도 달리한다: 빌링키는 서버가 SUBSCRIPTION_GRACE_DAYS(7일) 후 만료시키고,
// IAP 는 스토어 유예 최대치(App Store 최대 28일, Google Play 최대 30일)를 덮는다.
// (이상적으로는 스토어별 실제 유예 마감일을 서버가 노출해 그에 맞춰 판정해야 한다 — 후속 과제)
private val BILLING_KEY_GRACE_DURATION = 7.days
private val STORE_GRACE_DURATION = 31.days

sealed interface Entitlement {
  data object Unknown : Entitlement

  data class Active(val subscription: Subscription, val inGracePeriod: Boolean) : Entitlement

  data object Expired : Entitlement
}

// 권한 판정이 뒤집히는 시각. 유예 중이면 만료일이 아니라 유예 상한이 마감이므로,
// 시계 틱(오프라인 강등)도 이 시각에 맞춰 예약해야 한다.
fun entitlementDeadline(subscription: Subscription): Instant =
  if (subscription.state == SubscriptionState.IN_GRACE_PERIOD) {
    val graceBound =
      if (subscription.availability == PlanAvailability.BILLING_KEY) BILLING_KEY_GRACE_DURATION
      else STORE_GRACE_DURATION
    subscription.expiresAt + graceBound
  } else {
    subscription.expiresAt
  }

fun resolveEntitlement(subscription: Subscription?, now: Instant): Entitlement {
  if (subscription == null) {
    return Entitlement.Expired
  }
  if (subscription.state == SubscriptionState.IN_GRACE_PERIOD) {
    if (now <= entitlementDeadline(subscription)) {
      return Entitlement.Active(subscription = subscription, inGracePeriod = true)
    }
    return Entitlement.Expired
  }
  if (subscription.expiresAt <= now) {
    return Entitlement.Expired
  }
  return Entitlement.Active(subscription = subscription, inGracePeriod = false)
}
