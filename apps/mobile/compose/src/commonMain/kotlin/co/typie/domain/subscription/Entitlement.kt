package co.typie.domain.subscription

import co.typie.graphql.type.SubscriptionState
import kotlin.time.Instant

sealed interface Entitlement {
  data object Unknown : Entitlement

  data class Active(val subscription: Subscription, val inGracePeriod: Boolean) : Entitlement

  data object Expired : Entitlement
}

fun resolveEntitlement(subscription: Subscription?, now: Instant): Entitlement {
  if (subscription == null || subscription.expiresAt <= now) {
    return Entitlement.Expired
  }
  return Entitlement.Active(
    subscription = subscription,
    inGracePeriod = subscription.state == SubscriptionState.IN_GRACE_PERIOD,
  )
}
