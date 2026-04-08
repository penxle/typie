package co.typie.screen.subscription

import co.typie.di.Platform
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.type.buildUser
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.ApolloRequest
import com.apollographql.apollo.api.ApolloResponse
import com.apollographql.apollo.api.Operation
import com.apollographql.apollo.network.NetworkTransport
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.koin.core.context.startKoin
import org.koin.core.context.stopKoin
import org.koin.dsl.module
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalCoroutinesApi::class)
class CurrentPlanViewModelTest {
  private val dispatcher = StandardTestDispatcher()

  @BeforeTest
  fun setUp() {
    Dispatchers.setMain(dispatcher)
    stopKoin()
  }

  @AfterTest
  fun tearDown() {
    stopKoin()
    Dispatchers.resetMain()
  }

  @Test
  fun `subscription change refetches current plan query`() = runTest(dispatcher) {
    val networkTransport = CountingNetworkTransport()
    startKoin {
      modules(
        module {
          single<ApolloClient> {
            ApolloClient.Builder()
              .networkTransport(networkTransport)
              .build()
          }
        },
      )
    }

    val subscriptionSync = SubscriptionSync()
    CurrentPlanViewModel(
      subscriptionService = SubscriptionService(
        platform = Platform.iOS,
        purchaseService = CurrentPlanTestPurchaseService(),
        subscriptionDevSandbox = SubscriptionDevSandbox(Platform.iOS),
        subscriptionSync = subscriptionSync,
      ),
      subscriptionSync = subscriptionSync,
    )

    advanceUntilIdle()
    assertEquals(1, networkTransport.requestCount)

    subscriptionSync.notifyChanged()
    advanceUntilIdle()

    assertEquals(2, networkTransport.requestCount)
  }
}

private class CountingNetworkTransport : NetworkTransport {
  var requestCount = 0
    private set

  override fun <D : Operation.Data> execute(request: ApolloRequest<D>): Flow<ApolloResponse<D>> {
    requestCount += 1

    @Suppress("UNCHECKED_CAST")
    val data = currentPlanQueryData() as D

    return flowOf(
      ApolloResponse.Builder(
        operation = request.operation,
        requestUuid = request.requestUuid,
        data = data,
      )
        .isLast(true)
        .build(),
    )
  }

  override fun dispose() = Unit
}

private fun currentPlanQueryData() = CurrentPlanScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    credit = 0
    subscription = null
  }
}

private class CurrentPlanTestPurchaseService : co.typie.platform.PurchaseService {
  override val events = MutableSharedFlow<co.typie.platform.PurchaseEvent>()

  override suspend fun queryProducts(): Map<co.typie.platform.PurchasePlanInterval, co.typie.platform.PurchaseProduct> = emptyMap()

  override suspend fun purchase(
    product: co.typie.platform.PurchaseProduct,
    accountId: String,
  ): Boolean = false

  override suspend fun openSubscriptionManagement(): Boolean = false
}
