package co.typie.screen.subscription

import co.typie.dev.SubscriptionDevSandbox
import co.typie.dev.SubscriptionDevScenario
import co.typie.platform.Platform
import co.typie.graphql.QueryState
import co.typie.service.FULL_ACCESS_MONTHLY_PLAN_ID
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionManagementResult
import co.typie.service.SubscriptionService
import co.typie.service.SubscriptionSnapshot
import co.typie.service.SubscriptionState
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.platform.PurchaseService
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Instant

@OptIn(ExperimentalCoroutinesApi::class)
class SubscriptionServiceTest {
  @Test
  fun `desktop ignores query loading and error so entry screens can render from placeholder data`() {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop).apply {
      select(SubscriptionDevScenario.NoSubscription)
    }
    val service = SubscriptionService(
      platform = Platform.Desktop,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = sandbox,
    )

    assertFalse(service.hasQueryError(QueryState.Error(Exception("offline"))))
    assertFalse(service.isQueryLoading(QueryState.Loading))
  }

  @Test
  fun `desktop real data mode preserves remote query loading and error states`() {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop).apply {
      select(SubscriptionDevScenario.RemoteData)
    }
    val service = SubscriptionService(
      platform = Platform.Desktop,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = sandbox,
    )

    assertTrue(service.hasQueryError(QueryState.Error(Exception("offline"))))
    assertTrue(service.isQueryLoading(QueryState.Loading))
    assertFalse(service.isQueryLoading(QueryState.Success(remoteSubscriptionSnapshot())))
  }

  @Test
  fun `remote preserves query loading and error states`() {
    val service = SubscriptionService(
      platform = Platform.iOS,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = SubscriptionDevSandbox(Platform.iOS),
    )

    assertTrue(service.hasQueryError(QueryState.Error(Exception("offline"))))
    assertTrue(service.isQueryLoading(QueryState.Loading))
    assertFalse(service.isQueryLoading(QueryState.Success(remoteSubscriptionSnapshot())))
  }

  @Test
  fun `desktop canStartTrial reflects sandbox state instead of remote flag`() {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop).apply {
      select(SubscriptionDevScenario.NoSubscription)
    }
    val service = SubscriptionService(
      platform = Platform.Desktop,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = sandbox,
    )

    assertTrue(service.canStartTrial(remoteCanStartTrial = false))
  }

  @Test
  fun `desktop startTrial updates sandbox without running remote action`() = runTest {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop).apply {
      select(SubscriptionDevScenario.NoSubscription)
    }
    val service = SubscriptionService(
      platform = Platform.Desktop,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = sandbox,
    )
    var remoteCalled = false

    val celebration = service.startTrial {
      remoteCalled = true
    }

    assertFalse(remoteCalled)
    assertEquals(SubscriptionDevScenario.Trial, sandbox.scenario.value)
    assertEquals("무료 체험이 시작됐어요!", celebration.title)
  }

  @Test
  fun `remote startTrial runs remote action and does not mutate sandbox`() = runTest {
    val sandbox = SubscriptionDevSandbox(Platform.iOS)
    val service = SubscriptionService(
      platform = Platform.iOS,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = sandbox,
    )
    var remoteCalled = false

    val celebration = service.startTrial {
      remoteCalled = true
    }

    assertTrue(remoteCalled)
    assertEquals(SubscriptionDevScenario.RemoteData, sandbox.scenario.value)
    assertEquals("무료 체험이 시작됐어요!", celebration.title)
  }

  @Test
  fun `desktop purchase returns local celebration without calling purchase service`() = runTest {
    val purchaseService = FakePurchaseService()
    val sandbox = SubscriptionDevSandbox(Platform.Desktop).apply {
      select(SubscriptionDevScenario.NoSubscription)
    }
    val service = SubscriptionService(
      platform = Platform.Desktop,
      purchaseService = purchaseService,
      subscriptionDevSandbox = sandbox,
    )

    val result = service.purchase(
      product = fakeProduct(PurchasePlanInterval.Yearly),
      accountId = "user-uuid",
    )

    assertTrue(result.started)
    assertEquals("구독이 시작됐어요!", result.celebration?.title)
    assertEquals(SubscriptionDevScenario.Yearly, sandbox.scenario.value)
    assertEquals(0, purchaseService.purchaseCalls)
  }

  @Test
  fun `remote purchase delegates to purchase service and returns no immediate celebration`() = runTest {
    val purchaseService = FakePurchaseService(startPurchaseResult = true)
    val service = SubscriptionService(
      platform = Platform.Android,
      purchaseService = purchaseService,
      subscriptionDevSandbox = SubscriptionDevSandbox(Platform.Android),
    )

    val result = service.purchase(
      product = fakeProduct(PurchasePlanInterval.Monthly),
      accountId = "user-uuid",
    )

    assertTrue(result.started)
    assertNull(result.celebration)
    assertEquals(1, purchaseService.purchaseCalls)
  }

  @Test
  fun `desktop openSubscriptionManagement completes locally by scheduling cancel`() = runTest {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop).apply {
      purchase(PurchasePlanInterval.Monthly)
    }
    val service = SubscriptionService(
      platform = Platform.Desktop,
      purchaseService = FakePurchaseService(),
      subscriptionDevSandbox = sandbox,
    )

    val result = service.openSubscriptionManagement()

    assertEquals(SubscriptionManagementResult.CompletedLocally, result)
    assertEquals(SubscriptionDevScenario.CancelScheduled, sandbox.scenario.value)
  }

  @Test
  fun `remote openSubscriptionManagement waits for external store result`() = runTest {
    val purchaseService = FakePurchaseService(openSubscriptionManagementResult = true)
    val service = SubscriptionService(
      platform = Platform.Android,
      purchaseService = purchaseService,
      subscriptionDevSandbox = SubscriptionDevSandbox(Platform.Android),
    )

    val result = service.openSubscriptionManagement()

    assertEquals(SubscriptionManagementResult.AwaitingExternalResult, result)
    assertEquals(1, purchaseService.openSubscriptionManagementCalls)
  }
}

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

private fun fakeProduct(interval: PurchasePlanInterval): PurchaseProduct {
  return PurchaseProduct(
    id = "product-${interval.name.lowercase()}",
    interval = interval,
    price = "12,900원",
  )
}

private class FakePurchaseService(
  private val startPurchaseResult: Boolean = false,
  private val openSubscriptionManagementResult: Boolean = false,
) : PurchaseService {
  override val events: SharedFlow<PurchaseEvent> = MutableSharedFlow()

  var purchaseCalls = 0
    private set

  var openSubscriptionManagementCalls = 0
    private set

  override suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct> = emptyMap()

  override suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
  ): Boolean {
    purchaseCalls += 1
    return startPurchaseResult
  }

  override suspend fun openSubscriptionManagement(): Boolean {
    openSubscriptionManagementCalls += 1
    return openSubscriptionManagementResult
  }
}
