package co.typie.domain.subscription

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import co.typie.graphql.Apollo
import co.typie.graphql.QueryState
import co.typie.graphql.SubscriptionService_Query
import co.typie.graphql.watchQuery
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

sealed interface SubscriptionServiceState {
  data object Unknown : SubscriptionServiceState

  data object NotSubscribed : SubscriptionServiceState

  data class Subscribed(val subscription: Subscription) : SubscriptionServiceState
}

object SubscriptionService {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

  private val query = Apollo.watchQuery(scope = scope) { SubscriptionService_Query() }

  val state: SubscriptionServiceState by derivedStateOf {
    when (val raw = query.state) {
      QueryState.Loading,
      is QueryState.Error -> SubscriptionServiceState.Unknown
      is QueryState.Success -> {
        val subscription = raw.data.me.subscription?.toSubscription()
        if (subscription != null) {
          SubscriptionServiceState.Subscribed(subscription)
        } else {
          SubscriptionServiceState.NotSubscribed
        }
      }
    }
  }

  fun refresh() {
    query.refetch()
  }
}

private fun SubscriptionService_Query.Subscription.toSubscription(): Subscription {
  return Subscription(
    id = id,
    state = state,
    startsAt = startsAt,
    expiresAt = expiresAt,
    planId = plan.id,
    planName = plan.name,
    fee = plan.fee,
    availability = plan.availability,
  )
}
