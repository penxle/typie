package co.typie.graphql

import co.typie.platform.AppLifecycleService
import co.typie.platform.ConnectivityService
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.ApolloRequest
import com.apollographql.apollo.api.ApolloResponse
import com.apollographql.apollo.api.Operation
import com.apollographql.apollo.exception.ApolloException
import com.apollographql.apollo.exception.ApolloHttpException
import com.apollographql.apollo.exception.ApolloNetworkException
import com.apollographql.apollo.exception.ApolloOfflineException
import com.apollographql.apollo.exception.JsonDataException
import com.apollographql.apollo.network.NetworkTransport
import com.apollographql.cache.normalized.memory.MemoryCacheFactory
import com.apollographql.cache.normalized.normalizedCache
import kotlin.coroutines.CoroutineContext
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class WatchQueryTest {
  @Test
  fun foregroundReturnRetriesNetworkError() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val releaseRetry = CompletableDeferred<Unit>()
    val transport = TestNetworkTransport { attempt ->
      if (attempt == 2) releaseRetry.await()
      ApolloNetworkException("offline")
    }
    val query = watchQuery(transport, lifecycle)
    runCurrent()

    assertEquals(1, transport.attempts)
    assertIs<QueryState.Error>(query.state)

    lifecycle.update(foreground = false)
    lifecycle.update(foreground = true)
    runCurrent()

    assertIs<QueryState.Loading>(query.state)
    assertEquals(2, transport.attempts)
    releaseRetry.complete(Unit)
  }

  @Test
  fun connectivityRestorationRetriesOnlyInForeground() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val availability = MutableSharedFlow<Boolean>()
    val connectivity = ConnectivityService(availability)
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { connectivity.monitor() }
    availability.emit(true)
    val transport = TestNetworkTransport { ApolloNetworkException("offline") }
    val query = watchQuery(transport, lifecycle, connectivity)
    runCurrent()
    assertIs<QueryState.Error>(query.state)

    lifecycle.update(foreground = false)
    availability.emit(false)
    availability.emit(true)
    runCurrent()

    assertEquals(1, transport.attempts)

    lifecycle.update(foreground = true)
    runCurrent()

    assertEquals(2, transport.attempts)
  }

  @Test
  fun connectivityRestorationInForegroundRetriesNetworkError() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val availability = MutableSharedFlow<Boolean>()
    val connectivity = ConnectivityService(availability)
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) { connectivity.monitor() }
    availability.emit(true)
    val transport = TestNetworkTransport { ApolloNetworkException("offline") }
    val query = watchQuery(transport, lifecycle, connectivity)
    runCurrent()
    assertIs<QueryState.Error>(query.state)

    availability.emit(false)
    availability.emit(true)
    runCurrent()

    assertEquals(2, transport.attempts)
  }

  @Test
  fun foregroundReturnRetriesApolloOfflineError() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val transport = TestNetworkTransport { ApolloOfflineException() }
    val query = watchQuery(transport, lifecycle)
    runCurrent()
    assertIs<QueryState.Error>(query.state)

    lifecycle.update(foreground = false)
    lifecycle.update(foreground = true)
    runCurrent()

    assertEquals(2, transport.attempts)
  }

  @Test
  fun nonNetworkErrorsDoNotRecover() = runTest {
    val errors =
      listOf<ApolloException>(
        JsonDataException("invalid data"),
        ApolloHttpException(503, emptyList(), null, "unavailable"),
      )

    errors.forEach { error ->
      val lifecycle = AppLifecycleService().apply { update(foreground = true) }
      val transport = TestNetworkTransport { error }
      val query = watchQuery(transport, lifecycle)
      runCurrent()
      assertIs<QueryState.Error>(query.state)

      lifecycle.update(foreground = false)
      lifecycle.update(foreground = true)
      runCurrent()

      assertEquals(1, transport.attempts)
    }
  }

  @Test
  fun skippedQueryDoesNotRecover() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val transport = TestNetworkTransport { ApolloNetworkException("offline") }
    watchQuery(transport, lifecycle, skip = { true })
    runCurrent()

    lifecycle.update(foreground = false)
    lifecycle.update(foreground = true)
    runCurrent()

    assertEquals(0, transport.attempts)
  }

  @Test
  fun cancelledScopeDoesNotRecover() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val transport = TestNetworkTransport { ApolloNetworkException("offline") }
    val scope = CoroutineScope(StandardTestDispatcher(testScheduler) + SupervisorJob())
    val query = watchQuery(transport, lifecycle, scope = scope)
    runCurrent()
    assertIs<QueryState.Error>(query.state)

    scope.cancel()
    lifecycle.update(foreground = false)
    lifecycle.update(foreground = true)
    runCurrent()

    assertEquals(1, transport.attempts)
  }

  @Test
  fun recoveryDuringRequestRetriesOnceThenStopsWithoutNewEvent() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val releaseFirst = CompletableDeferred<Unit>()
    val transport = TestNetworkTransport { attempt ->
      if (attempt == 1) releaseFirst.await()
      ApolloNetworkException("offline")
    }
    val query = watchQuery(transport, lifecycle)
    runCurrent()

    assertIs<QueryState.Loading>(query.state)
    assertEquals(1, transport.attempts)

    lifecycle.update(foreground = false)
    lifecycle.update(foreground = true)
    runCurrent()
    releaseFirst.complete(Unit)
    runCurrent()

    assertIs<QueryState.Error>(query.state)
    assertEquals(2, transport.attempts)
    runCurrent()
    assertEquals(2, transport.attempts)
  }

  @Test
  fun nonMainParentCannotOwnQueryStateTransitions() = runTest {
    val lifecycle = AppLifecycleService().apply { update(foreground = true) }
    val parentScope = CoroutineScope(SupervisorJob() + NeverDispatcher)
    val transport = TestNetworkTransport { ApolloNetworkException("offline") }
    val query = watchQuery(transport, lifecycle, scope = parentScope)
    runCurrent()

    assertIs<QueryState.Error>(query.state)
    assertEquals(1, transport.attempts)

    lifecycle.update(foreground = false)
    lifecycle.update(foreground = true)
    runCurrent()

    assertIs<QueryState.Error>(query.state)
    assertEquals(2, transport.attempts)
    parentScope.cancel()
  }

  private fun TestScope.watchQuery(
    transport: TestNetworkTransport,
    lifecycle: AppLifecycleService,
    connectivity: ConnectivityService = ConnectivityService(emptyFlow()),
    scope: CoroutineScope = backgroundScope,
    skip: () -> Boolean = { false },
  ): WatchQuery<PlanUpgradeSheet_Query.Data, PlanUpgradeSheet_Query.Data?> {
    val client =
      ApolloClient.Builder()
        .networkTransport(transport)
        .dispatcher(StandardTestDispatcher(testScheduler))
        .normalizedCache(
          MemoryCacheFactory(maxSizeBytes = 1024 * 1024),
          typePolicies = emptyMap(),
          fieldPolicies = emptyMap(),
        )
        .build()
    return WatchQuery(
      scope = scope,
      apolloClient = client,
      query = ::PlanUpgradeSheet_Query,
      placeholderData = null,
      onInitialData = null,
      skip = skip,
      resetOnChange = true,
      appLifecycleService = lifecycle,
      connectivityService = connectivity,
      stateDispatcher = StandardTestDispatcher(testScheduler),
    )
  }
}

private object NeverDispatcher : CoroutineDispatcher() {
  override fun dispatch(context: CoroutineContext, block: Runnable) {}
}

private class TestNetworkTransport(
  private val response: suspend (attempt: Int) -> ApolloException
) : NetworkTransport {
  var attempts = 0
    private set

  override fun <D : Operation.Data> execute(request: ApolloRequest<D>): Flow<ApolloResponse<D>> =
    flow {
      val exception = response(++attempts)
      emit(
        ApolloResponse.Builder(request.operation, request.requestUuid)
          .exception(exception)
          .isLast(true)
          .build()
      )
    }

  override fun dispose() {}
}
