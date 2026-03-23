package co.typie.graphql

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.touchlab.kermit.Logger
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Query
import com.apollographql.apollo.cache.normalized.watch
import com.apollographql.apollo.exception.CacheMissException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.launch

class WatchQuery<D : Query.Data, out R> private constructor(
  private val scope: CoroutineScope,
  private val apolloClient: ApolloClient,
  private val query: () -> Query<D>,
  private val placeholderData: D?,
  private val onInitialData: ((D) -> Unit)?,
) {
  var state: QueryState<D> by mutableStateOf(QueryState.Loading)
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
    ): WatchQuery<D, D?> = WatchQuery(scope, apolloClient, query, null, onInitialData)

    operator fun <D : Query.Data> invoke(
      scope: CoroutineScope,
      apolloClient: ApolloClient,
      query: () -> Query<D>,
      placeholderData: D,
      onInitialData: ((D) -> Unit)? = null,
    ): WatchQuery<D, D> = WatchQuery(scope, apolloClient, query, placeholderData, onInitialData)
  }

  private var job: Job? = null
  private var initialized = false

  init {
    scope.launch {
      snapshotFlow { query() }
        .distinctUntilChanged()
        .collect { query -> execute(query, resetState = true) }
    }
  }

  private fun execute(query: Query<D>, resetState: Boolean = false) {
    job?.cancel()

    if (resetState || state !is QueryState.Success) {
      state = QueryState.Loading
    }

    job = scope.launch {
      try {
        apolloClient.query(query).watch().collect { response ->
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
              Logger.e(error) { "GraphQL error" }
              state = QueryState.Error(error)
            }
          }
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "GraphQL error" }
        state = QueryState.Error(e)
      }
    }
  }

  fun refetch() {
    execute(query())
  }
}
