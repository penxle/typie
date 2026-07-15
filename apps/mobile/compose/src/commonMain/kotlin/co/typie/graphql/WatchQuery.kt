package co.typie.graphql

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.touchlab.kermit.Logger
import co.typie.contract.Loadable
import co.typie.network.isRecoverableNetworkError
import co.typie.platform.AppLifecycleService
import co.typie.platform.AppLifecycleState
import co.typie.platform.ConnectivityService
import co.typie.platform.appLifecycleService as defaultAppLifecycleService
import co.typie.platform.connectivityService as defaultConnectivityService
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Query
import com.apollographql.apollo.exception.CacheMissException
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy
import com.apollographql.cache.normalized.refetchPolicyInterceptor
import com.apollographql.cache.normalized.watch
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

class WatchQuery<D : Query.Data, out R>
internal constructor(
  scope: CoroutineScope,
  private val apolloClient: ApolloClient,
  private val query: () -> Query<D>,
  private val placeholderData: D?,
  private val onInitialData: (suspend (D) -> Unit)?,
  private val skip: () -> Boolean = { false },
  private val resetOnChange: Boolean = true,
  private val appLifecycleService: AppLifecycleService = defaultAppLifecycleService,
  private val connectivityService: ConnectivityService = defaultConnectivityService,
  stateDispatcher: CoroutineDispatcher = Dispatchers.Main.immediate,
) : Loadable<D> {
  override var state: QueryState<D> by mutableStateOf(QueryState.Loading)
    private set

  /**
   * Query represented by [state]. When `resetOnChange` is false, a new in-flight query does not
   * replace this value until it publishes data or an error.
   */
  var stateQuery: Query<D>? by mutableStateOf(null)
    private set

  @Suppress("UNCHECKED_CAST")
  val data: R
    get() = ((state as? QueryState.Success)?.data ?: placeholderData) as R

  companion object {
    operator fun <D : Query.Data> invoke(
      scope: CoroutineScope,
      apolloClient: ApolloClient,
      query: () -> Query<D>,
      onInitialData: (suspend (D) -> Unit)? = null,
      skip: () -> Boolean = { false },
      resetOnChange: Boolean = true,
    ): WatchQuery<D, D?> =
      WatchQuery(scope, apolloClient, query, null, onInitialData, skip, resetOnChange)

    operator fun <D : Query.Data> invoke(
      scope: CoroutineScope,
      apolloClient: ApolloClient,
      query: () -> Query<D>,
      placeholderData: D,
      onInitialData: (suspend (D) -> Unit)? = null,
      skip: () -> Boolean = { false },
      resetOnChange: Boolean = true,
    ): WatchQuery<D, D> =
      WatchQuery(scope, apolloClient, query, placeholderData, onInitialData, skip, resetOnChange)
  }

  private var job: Job? = null
  private var initialized = false
  private var recoveryGeneration = 0L
  private val stateScope = CoroutineScope(scope.coroutineContext + stateDispatcher)

  init {
    stateScope.launch {
      snapshotFlow {
        val shouldSkip = skip()
        shouldSkip to if (shouldSkip) null else query()
      }
        .distinctUntilChanged()
        .collect { (shouldSkip, query) ->
          if (shouldSkip) {
            job?.cancel()
          } else if (query != null) {
            execute(query, resetState = resetOnChange)
          }
        }
    }

    stateScope.launch {
      var foregroundGeneration = appLifecycleService.snapshot.value.foregroundGeneration
      var connectivityGeneration = connectivityService.restorationGeneration.value
      combine(appLifecycleService.snapshot, connectivityService.restorationGeneration) {
          lifecycle,
          connectivity ->
          lifecycle to connectivity
        }
        .collect { (lifecycle, connectivity) ->
          val returnedToForeground = lifecycle.foregroundGeneration > foregroundGeneration
          val restoredInForeground =
            connectivity > connectivityGeneration && lifecycle.state == AppLifecycleState.Foreground
          foregroundGeneration = lifecycle.foregroundGeneration
          connectivityGeneration = connectivity

          if (returnedToForeground || restoredInForeground) {
            recoveryGeneration += 1
            val error = (state as? QueryState.Error)?.exception
            if (error?.isRecoverableNetworkError() == true) {
              refetch()
            }
          }
        }
    }
  }

  private fun execute(query: Query<D>, resetState: Boolean = false) {
    job?.cancel()

    if (resetState || state !is QueryState.Success) {
      stateQuery = query
      state = QueryState.Loading
    }

    val attemptRecoveryGeneration = recoveryGeneration
    job = stateScope.launch {
      try {
        apolloClient
          .query(query)
          .fetchPolicy(FetchPolicy.CacheAndNetwork)
          .refetchPolicyInterceptor(CacheFirstRefetchInterceptor)
          .watch()
          .collect { response ->
            val data = response.data
            if (data != null) {
              stateQuery = query
              state = QueryState.Success(data)
              if (!initialized) {
                initialized = true
                onInitialData?.invoke(data)
              }
            } else if (response.exception is CacheMissException) {
              // CacheAndNetwork 정책에서 캐시가 비어있을 때 — 네트워크 응답을 기다림
            } else {
              val error =
                response.exception ?: response.errors?.firstOrNull()?.let { Exception(it.message) }
              if (error != null) {
                publishError(query, error, attemptRecoveryGeneration)
              }
            }
          }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        publishError(query, e, attemptRecoveryGeneration)
      }
    }
  }

  private fun publishError(query: Query<D>, error: Throwable, attemptRecoveryGeneration: Long) {
    stateQuery = query
    state = QueryState.Error(error)
    runCatching {
      Logger.e { "GraphQL error (${query.name()}): ${error.message ?: "unknown error"}" }
    }
    if (!error.isRecoverableNetworkError()) {
      runCatching { Sentry.captureException(error) }
    }

    if (error.isRecoverableNetworkError() && recoveryGeneration > attemptRecoveryGeneration) {
      refetch()
    }
  }

  override fun refetch() {
    stateScope.launch {
      if (skip()) {
        return@launch
      }

      execute(query())
    }
  }
}

fun <D : Query.Data> ApolloClient.watchQuery(
  scope: CoroutineScope,
  onInitialData: (suspend (D) -> Unit)? = null,
  skip: () -> Boolean = { false },
  resetOnChange: Boolean = true,
  query: () -> Query<D>,
): WatchQuery<D, D?> =
  WatchQuery(
    scope,
    this,
    query,
    onInitialData = onInitialData,
    skip = skip,
    resetOnChange = resetOnChange,
  )

fun <D : Query.Data> ApolloClient.watchQuery(
  scope: CoroutineScope,
  placeholderData: D,
  onInitialData: (suspend (D) -> Unit)? = null,
  skip: () -> Boolean = { false },
  resetOnChange: Boolean = true,
  query: () -> Query<D>,
): WatchQuery<D, D> =
  WatchQuery(scope, this, query, placeholderData, onInitialData, skip, resetOnChange)
