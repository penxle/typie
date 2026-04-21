package co.typie.graphql

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.touchlab.kermit.Logger
import co.typie.contract.Loadable
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Query
import com.apollographql.apollo.exception.CacheMissException
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy
import com.apollographql.cache.normalized.refetchPolicyInterceptor
import com.apollographql.cache.normalized.watch
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

class WatchQuery<D : Query.Data, out R>
private constructor(
  private val scope: CoroutineScope,
  private val apolloClient: ApolloClient,
  private val query: () -> Query<D>,
  private val placeholderData: D?,
  private val onInitialData: ((D) -> Unit)?,
  private val skip: () -> Boolean = { false },
  private val resetOnChange: Boolean = true,
) : Loadable<D> {
  override var state: QueryState<D> by mutableStateOf(QueryState.Loading)
    private set

  @Suppress("UNCHECKED_CAST")
  val data: R
    get() = ((state as? QueryState.Success)?.data ?: placeholderData) as R

  companion object {
    operator fun <D : Query.Data> invoke(
      scope: CoroutineScope,
      apolloClient: ApolloClient,
      query: () -> Query<D>,
      onInitialData: ((D) -> Unit)? = null,
      skip: () -> Boolean = { false },
      resetOnChange: Boolean = true,
    ): WatchQuery<D, D?> =
      WatchQuery(scope, apolloClient, query, null, onInitialData, skip, resetOnChange)

    operator fun <D : Query.Data> invoke(
      scope: CoroutineScope,
      apolloClient: ApolloClient,
      query: () -> Query<D>,
      placeholderData: D,
      onInitialData: ((D) -> Unit)? = null,
      skip: () -> Boolean = { false },
      resetOnChange: Boolean = true,
    ): WatchQuery<D, D> =
      WatchQuery(scope, apolloClient, query, placeholderData, onInitialData, skip, resetOnChange)
  }

  private var job: Job? = null
  private var initialized = false

  init {
    scope.launch {
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
  }

  private fun execute(query: Query<D>, resetState: Boolean = false) {
    job?.cancel()

    if (resetState || state !is QueryState.Success) {
      state = QueryState.Loading
    }

    job = scope.launch {
      try {
        apolloClient
          .query(query)
          .fetchPolicy(FetchPolicy.CacheAndNetwork)
          .refetchPolicyInterceptor(CacheFirstRefetchInterceptor)
          .watch()
          .collect { response ->
            val data = response.data
            if (data != null) {
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
                Logger.e(error) { "GraphQL error (${query.name()})" }
                Sentry.captureException(error)
                state = QueryState.Error(error)
              }
            }
          }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "GraphQL error" }
        Sentry.captureException(e)
        state = QueryState.Error(e)
      }
    }
  }

  override fun refetch() {
    execute(query())
  }
}

fun <D : Query.Data> ApolloClient.watchQuery(
  scope: CoroutineScope,
  onInitialData: ((D) -> Unit)? = null,
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
  onInitialData: ((D) -> Unit)? = null,
  skip: () -> Boolean = { false },
  resetOnChange: Boolean = true,
  query: () -> Query<D>,
): WatchQuery<D, D> =
  WatchQuery(scope, this, query, placeholderData, onInitialData, skip, resetOnChange)
