package co.typie.screen.subscription

import co.typie.dev.SubscriptionDevSandbox
import co.typie.dev.SubscriptionDevScenario
import co.typie.graphql.QueryState
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.service.FULL_ACCESS_MONTHLY_PLAN_ID
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionManagementResult
import co.typie.service.SubscriptionService
import co.typie.service.SubscriptionSnapshot
import co.typie.service.SubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.coroutines.test.runTest

class SubscriptionServiceTest {
  @Test
  fun `desktop ignores query loading and error while sandbox is active`() {
    withSubscriptionScenario(SubscriptionDevScenario.NoSubscription) {
      assertFalse(SubscriptionService.hasQueryError(QueryState.Error(Exception("offline"))))
      assertFalse(SubscriptionService.isQueryLoading(QueryState.Loading))
    }
  }

  @Test
  fun `desktop real data mode preserves remote query loading and error states`() {
    withSubscriptionScenario(SubscriptionDevScenario.RemoteData) {
      assertTrue(SubscriptionService.hasQueryError(QueryState.Error(Exception("offline"))))
      assertTrue(SubscriptionService.isQueryLoading(QueryState.Loading))
      assertFalse(
        SubscriptionService.isQueryLoading(QueryState.Success(remoteSubscriptionSnapshot()))
      )
    }
  }

  @Test
  fun `desktop canStartTrial reflects sandbox state instead of remote flag`() {
    withSubscriptionScenario(SubscriptionDevScenario.NoSubscription) {
      assertTrue(SubscriptionService.canStartTrial(remoteCanStartTrial = false))
    }
  }

  @Test
  fun `desktop startTrial updates sandbox without running remote action`() = runTest {
    withSubscriptionScenarioSuspending(SubscriptionDevScenario.NoSubscription) {
      var remoteCalled = false

      val celebration = SubscriptionService.startTrial { remoteCalled = true }

      assertFalse(remoteCalled)
      assertEquals(SubscriptionDevScenario.Trial, SubscriptionDevSandbox.scenario)
      assertEquals("무료 체험이 시작됐어요!", celebration.title)
    }
  }

  @Test
  fun `desktop purchase returns local celebration while sandbox is active`() = runTest {
    withSubscriptionScenarioSuspending(SubscriptionDevScenario.NoSubscription) {
      val result =
        SubscriptionService.purchase(
          product = fakeProduct(PurchasePlanInterval.Monthly),
          accountId = "user-uuid",
        )

      assertTrue(result.started)
      assertEquals("구독이 시작됐어요!", result.celebration?.title)
      assertEquals(SubscriptionDevScenario.Monthly, SubscriptionDevSandbox.scenario)
    }
  }

  @Test
  fun `desktop openSubscriptionManagement completes locally by scheduling cancel`() = runTest {
    withSubscriptionScenarioSuspending(SubscriptionDevScenario.Monthly) {
      val result = SubscriptionService.openSubscriptionManagement()

      assertEquals(SubscriptionManagementResult.CompletedLocally, result)
      assertEquals(SubscriptionDevScenario.CancelScheduled, SubscriptionDevSandbox.scenario)
    }
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

private fun <T> withSubscriptionScenario(initial: SubscriptionDevScenario, block: () -> T): T {
  val previous = SubscriptionDevSandbox.scenario
  SubscriptionDevSandbox.select(initial)
  return try {
    block()
  } finally {
    SubscriptionDevSandbox.select(previous)
  }
}

private suspend fun <T> withSubscriptionScenarioSuspending(
  initial: SubscriptionDevScenario,
  block: suspend () -> T,
): T {
  val previous = SubscriptionDevSandbox.scenario
  SubscriptionDevSandbox.select(initial)
  return try {
    block()
  } finally {
    SubscriptionDevSandbox.select(previous)
  }
}
