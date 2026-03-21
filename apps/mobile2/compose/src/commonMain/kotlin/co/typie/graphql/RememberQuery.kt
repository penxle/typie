package co.typie.graphql

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import co.touchlab.kermit.Logger
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.Query
import com.apollographql.apollo.cache.normalized.watch
import com.apollographql.apollo.exception.CacheMissException
import org.koin.compose.koinInject
import kotlin.coroutines.cancellation.CancellationException

sealed interface QueryState<out D> {
  data object Loading : QueryState<Nothing>
  data class Success<D>(val data: D) : QueryState<D>
  data class Error(val exception: Throwable) : QueryState<Nothing>
}

data class QueryResult<D>(
  val state: QueryState<D>,
  val refetch: () -> Unit,
)

internal class QueryStateHolder : ViewModel() {
  private val states = mutableMapOf<Any, MutableState<QueryState<*>>>()
  private val refreshKeys = mutableMapOf<Any, MutableState<Int>>()

  @Suppress("UNCHECKED_CAST")
  fun <D> stateFor(key: Any): MutableState<QueryState<D>> =
    states.getOrPut(key) { mutableStateOf(QueryState.Loading) } as MutableState<QueryState<D>>

  fun refreshKeyFor(key: Any): MutableState<Int> =
    refreshKeys.getOrPut(key) { mutableStateOf(0) }
}

@Composable
fun <D : Query.Data> rememberQuery(query: Query<D>): QueryResult<D> {
  val apolloClient = koinInject<ApolloClient>()
  val holder = viewModel { QueryStateHolder() }
  var state by holder.stateFor<D>(query)
  var refreshKey by holder.refreshKeyFor(query)

  LaunchedEffect(query, refreshKey) {
    try {
      apolloClient.query(query).watch().collect { response ->
        val data = response.data
        if (data != null) {
          state = QueryState.Success(data)
        } else if (response.exception is CacheMissException) {
          // CacheAndNetwork 정책에서 캐시가 비어있을 때 발생 — 네트워크 응답을 기다림
        } else {
          val error =
            response.exception ?: response.errors?.firstOrNull()?.let { Exception(it.message) }
          Logger.e(error) { "GraphQL error" }
          if (error != null) {
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

  return QueryResult(
    state = state,
    refetch = { refreshKey++ },
  )
}
