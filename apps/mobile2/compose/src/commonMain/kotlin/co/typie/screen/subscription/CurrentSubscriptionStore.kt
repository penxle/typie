package co.typie.screen.subscription

import co.touchlab.kermit.Logger
import co.typie.di.Platform
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.QueryState
import com.apollographql.apollo.ApolloClient
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy
import com.apollographql.cache.normalized.watch
import com.apollographql.apollo.exception.CacheMissException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import org.koin.core.annotation.Single

@Single
class CurrentSubscriptionStore(
  private val apolloClient: ApolloClient,
  private val platform: Platform,
  private val subscriptionDevSandbox: SubscriptionDevSandbox,
) {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private val remoteState = MutableStateFlow<QueryState<SubscriptionSnapshot?>>(QueryState.Loading)
  private var remoteWatchJob: Job? = null

  val state: StateFlow<QueryState<SubscriptionSnapshot?>> = combine(
    subscriptionDevSandbox.scenario,
    remoteState,
  ) { scenario, currentRemoteState ->
    effectiveCurrentSubscriptionState(
      platform = platform,
      scenario = scenario,
      remoteState = currentRemoteState,
    )
  }.stateIn(
    scope = scope,
    started = SharingStarted.Eagerly,
    initialValue = effectiveCurrentSubscriptionState(
      platform = platform,
      scenario = subscriptionDevSandbox.scenario.value,
      remoteState = remoteState.value,
    ),
  )

  val usesSandbox: Boolean
    get() = platform == Platform.Desktop && subscriptionDevSandbox.usesSandbox

  init {
    scope.launch {
      subscriptionDevSandbox.scenario
        .map { scenario -> platform == Platform.Desktop && scenario != SubscriptionDevScenario.RemoteData }
        .distinctUntilChanged()
        .collect { useSandbox ->
          if (useSandbox) {
            remoteWatchJob?.cancel()
            remoteWatchJob = null
          } else {
            startRemoteWatch(resetState = true)
          }
        }
    }
  }

  fun refresh() {
    if (usesSandbox) return

    scope.launch {
      try {
        apolloClient.query(CurrentPlanScreen_Query())
          .fetchPolicy(FetchPolicy.NetworkOnly)
          .execute()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to refresh current subscription" }
        remoteState.value = QueryState.Error(e)
      }
    }
  }

  private fun startRemoteWatch(resetState: Boolean) {
    remoteWatchJob?.cancel()

    if (resetState || remoteState.value !is QueryState.Success) {
      remoteState.value = QueryState.Loading
    }

    remoteWatchJob = scope.launch {
      try {
        apolloClient.query(CurrentPlanScreen_Query()).watch().collect { response ->
          val data = response.data
          if (data != null) {
            val subscription = data.me.subscription?.toSubscriptionSnapshot()
            remoteState.value = QueryState.Success(subscription)
          } else if (response.exception is CacheMissException) {
            // CacheAndNetwork policy can emit an empty cached response before network data arrives.
          } else {
            val error = response.exception ?: response.errors?.firstOrNull()?.let { Exception(it.message) }
            if (error != null) {
              Logger.e(error) { "Failed to watch current subscription" }
              remoteState.value = QueryState.Error(error)
            }
          }
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to watch current subscription" }
        remoteState.value = QueryState.Error(e)
      }
    }
  }
}

private fun effectiveCurrentSubscriptionState(
  platform: Platform,
  scenario: SubscriptionDevScenario,
  remoteState: QueryState<SubscriptionSnapshot?>,
): QueryState<SubscriptionSnapshot?> {
  return if (platform == Platform.Desktop && scenario != SubscriptionDevScenario.RemoteData) {
    QueryState.Success(subscriptionDevSubscription(scenario))
  } else {
    remoteState
  }
}
