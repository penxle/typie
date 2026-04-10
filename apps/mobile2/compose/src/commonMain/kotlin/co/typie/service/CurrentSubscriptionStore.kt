package co.typie.service

import co.touchlab.kermit.Logger
import co.typie.graphql.Apollo
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.QueryState
import co.typie.dev.SubscriptionDevScenario
import co.typie.dev.SubscriptionDevSandbox
import co.typie.dev.subscriptionDevSubscription
import co.typie.graphql.toSubscriptionSnapshot
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

object CurrentSubscriptionStore {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private val remoteState = MutableStateFlow<QueryState<SubscriptionSnapshot?>>(QueryState.Loading)
  private var remoteWatchJob: Job? = null

  val state: StateFlow<QueryState<SubscriptionSnapshot?>> = combine(
    SubscriptionDevSandbox.scenario,
    remoteState,
  ) { scenario, currentRemoteState ->
    effectiveCurrentSubscriptionState(
      platform = PlatformModule.platform,
      scenario = scenario,
      remoteState = currentRemoteState,
    )
  }.stateIn(
    scope = scope,
    started = SharingStarted.Eagerly,
    initialValue = effectiveCurrentSubscriptionState(
      platform = PlatformModule.platform,
      scenario = SubscriptionDevSandbox.scenario.value,
      remoteState = remoteState.value,
    ),
  )

  val usesSandbox: Boolean
    get() = PlatformModule.platform == Platform.Desktop && SubscriptionDevSandbox.usesSandbox

  init {
    scope.launch {
      SubscriptionDevSandbox.scenario
        .map { scenario -> PlatformModule.platform == Platform.Desktop && scenario != SubscriptionDevScenario.RemoteData }
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
        Apollo.query(CurrentPlanScreen_Query())
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
        Apollo.query(CurrentPlanScreen_Query()).watch().collect { response ->
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
