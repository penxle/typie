package co.typie.screen.subscription

import co.typie.dev.SubscriptionDevSandbox
import co.typie.dev.SubscriptionDevScenario
import co.typie.graphql.CurrentPlanScreen_Query
import co.typie.graphql.QueryState
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState as GraphqlSubscriptionState
import co.typie.platform.Platform
import co.typie.service.CurrentSubscriptionStore
import co.typie.service.FULL_ACCESS_MONTHLY_PLAN_ID
import co.typie.service.FULL_ACCESS_YEARLY_PLAN_ID
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionSnapshot
import co.typie.service.SubscriptionState
import co.typie.service.hasSubscriptionOrNull
import co.typie.service.subscriptionSummaryOrNull
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.ApolloRequest
import com.apollographql.apollo.api.ApolloResponse
import com.apollographql.apollo.api.Operation
import com.apollographql.apollo.network.NetworkTransport
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.yield

@OptIn(ExperimentalCoroutinesApi::class)
class CurrentSubscriptionStoreTest {
  @Test
  fun `remote store loads current subscription summary`() = runTest {
    val networkTransport =
      CurrentSubscriptionNetworkTransport(data = currentPlanQueryData(remoteSubscriptionSnapshot()))
    val store =
      CurrentSubscriptionStore(
        apolloClient = ApolloClient.Builder().networkTransport(networkTransport).build(),
        platform = Platform.iOS,
        subscriptionDevSandbox = SubscriptionDevSandbox(Platform.iOS),
      )

    val state = awaitSuccessState(store)

    assertEquals(FULL_ACCESS_MONTHLY_PLAN_ID, state.data?.planId)
    assertEquals("원격 FULL ACCESS", state.subscriptionSummaryOrNull()?.subscriptionName)
    assertEquals(true, state.hasSubscriptionOrNull())
    assertEquals(1, networkTransport.requestCount)
  }

  @Test
  fun `desktop sandbox store reflects scenario changes without remote fetch`() = runTest {
    val networkTransport =
      CurrentSubscriptionNetworkTransport(data = currentPlanQueryData(remoteSubscriptionSnapshot()))
    val sandbox =
      SubscriptionDevSandbox(Platform.Desktop).apply {
        select(SubscriptionDevScenario.NoSubscription)
      }
    val store =
      CurrentSubscriptionStore(
        apolloClient = ApolloClient.Builder().networkTransport(networkTransport).build(),
        platform = Platform.Desktop,
        subscriptionDevSandbox = sandbox,
      )

    val initialState = store.state.value
    assertTrue(initialState is QueryState.Success)
    assertEquals(null, initialState.data)
    assertEquals(0, networkTransport.requestCount)

    sandbox.purchase(co.typie.platform.PurchasePlanInterval.Yearly)

    val updatedState =
      withTimeout(1_000) {
        while (true) {
          val current = store.state.value
          if (current is QueryState.Success && current.data?.planId == FULL_ACCESS_YEARLY_PLAN_ID) {
            return@withTimeout current
          }
          yield()
        }
        error("Timed out waiting for yearly subscription state")
      }

    assertEquals(FULL_ACCESS_YEARLY_PLAN_ID, updatedState.data?.planId)
    assertEquals("타이피 FULL ACCESS", updatedState.subscriptionSummaryOrNull()?.subscriptionName)
    assertEquals(0, networkTransport.requestCount)
  }

  @Test
  fun `remote refresh performs another current subscription request`() = runTest {
    val networkTransport =
      CurrentSubscriptionNetworkTransport(data = currentPlanQueryData(remoteSubscriptionSnapshot()))
    val store =
      CurrentSubscriptionStore(
        apolloClient = ApolloClient.Builder().networkTransport(networkTransport).build(),
        platform = Platform.iOS,
        subscriptionDevSandbox = SubscriptionDevSandbox(Platform.iOS),
      )

    withTimeout(1_000) {
      while (store.state.value !is QueryState.Success) {
        yield()
      }
    }
    assertEquals(1, networkTransport.requestCount)

    store.refresh()

    withTimeout(1_000) {
      while (networkTransport.requestCount < 2) {
        yield()
      }
    }

    assertEquals(2, networkTransport.requestCount)
  }
}

private class CurrentSubscriptionNetworkTransport(var data: CurrentPlanScreen_Query.Data) :
  NetworkTransport {
  var requestCount = 0
    private set

  override fun <D : Operation.Data> execute(request: ApolloRequest<D>): Flow<ApolloResponse<D>> {
    requestCount += 1

    @Suppress("UNCHECKED_CAST") val responseData = data as D

    return flowOf(
      ApolloResponse.Builder(
          operation = request.operation,
          requestUuid = request.requestUuid,
          data = responseData,
        )
        .isLast(true)
        .build()
    )
  }

  override fun dispose() = Unit
}

private suspend fun awaitSuccessState(
  store: CurrentSubscriptionStore
): QueryState.Success<SubscriptionSnapshot?> {
  return withTimeout(1_000) {
    while (true) {
      val current = store.state.value
      if (current is QueryState.Success) {
        return@withTimeout current
      }
      yield()
    }
    error("Timed out waiting for current subscription state")
  }
}

private fun currentPlanQueryData(subscription: SubscriptionSnapshot?) =
  CurrentPlanScreen_Query.Data(
    me =
      CurrentPlanScreen_Query.Me(
        __typename = "User",
        id = "user-id",
        credit = 0,
        subscription =
          subscription?.let {
            CurrentPlanScreen_Query.Subscription(
              __typename = "Subscription",
              id = it.id ?: "subscription-id",
              state =
                when (it.state) {
                  SubscriptionState.Canceled -> GraphqlSubscriptionState.WILL_EXPIRE
                  SubscriptionState.Active,
                  null -> GraphqlSubscriptionState.ACTIVE
                },
              startsAt = it.startsAt ?: Instant.parse("2026-03-29T00:00:00Z"),
              expiresAt = it.expiresAt ?: Instant.parse("2026-04-29T00:00:00Z"),
              plan =
                CurrentPlanScreen_Query.Plan(
                  __typename = "Plan",
                  id = it.planId ?: FULL_ACCESS_MONTHLY_PLAN_ID,
                  name = it.planName ?: "원격 FULL ACCESS",
                  fee = it.fee ?: 12_900,
                  availability =
                    when (it.availability) {
                      SubscriptionAvailability.BillingKey -> PlanAvailability.BILLING_KEY
                      SubscriptionAvailability.Manual -> PlanAvailability.MANUAL
                      SubscriptionAvailability.Trial -> PlanAvailability.TRIAL
                      SubscriptionAvailability.InAppPurchase,
                      null -> PlanAvailability.IN_APP_PURCHASE
                    },
                ),
            )
          },
      )
  )

private fun remoteSubscriptionSnapshot(): SubscriptionSnapshot {
  return SubscriptionSnapshot(
    id = "remote-subscription",
    state = SubscriptionState.Active,
    startsAt = Instant.parse("2026-03-29T00:00:00Z"),
    expiresAt = Instant.parse("2026-04-29T00:00:00Z"),
    planId = FULL_ACCESS_MONTHLY_PLAN_ID,
    planName = "원격 FULL ACCESS",
    fee = 12_900,
    availability = SubscriptionAvailability.InAppPurchase,
  )
}
