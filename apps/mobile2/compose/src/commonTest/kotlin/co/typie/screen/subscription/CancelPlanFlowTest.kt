package co.typie.screen.subscription

import co.typie.service.CancelPlanFlowState
import co.typie.service.OPEN_SUBSCRIPTION_MANAGEMENT_FAILURE_MESSAGE
import co.typie.service.SubscriptionManagementResult
import co.typie.service.SubscriptionState
import co.typie.service.consumeCancelPlanCloseRequest
import co.typie.service.consumeCancelPlanErrorMessage
import co.typie.service.reduceCancelPlanFlowOnManagementResult
import co.typie.service.reduceCancelPlanFlowOnSubscriptionState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CancelPlanFlowTest {
  @Test
  fun `failed store open keeps screen open and exposes an error message`() {
    val state =
      reduceCancelPlanFlowOnManagementResult(
        current = CancelPlanFlowState(),
        result = SubscriptionManagementResult.FailedToOpen,
      )

    assertFalse(state.awaitingStoreResult)
    assertFalse(state.shouldClose)
    assertEquals(OPEN_SUBSCRIPTION_MANAGEMENT_FAILURE_MESSAGE, state.errorMessage)
  }

  @Test
  fun `awaiting state starts when subscription management moves to external store`() {
    val state =
      reduceCancelPlanFlowOnManagementResult(
        current = CancelPlanFlowState(),
        result = SubscriptionManagementResult.AwaitingExternalResult,
      )

    assertTrue(state.awaitingStoreResult)
    assertFalse(state.shouldClose)
  }

  @Test
  fun `close is requested immediately when subscription management completes locally`() {
    val state =
      reduceCancelPlanFlowOnManagementResult(
        current = CancelPlanFlowState(),
        result = SubscriptionManagementResult.CompletedLocally,
      )

    assertFalse(state.awaitingStoreResult)
    assertTrue(state.shouldClose)
  }

  @Test
  fun `subscription refresh closes screen after external cancellation is reflected`() {
    val state =
      reduceCancelPlanFlowOnSubscriptionState(
        current = CancelPlanFlowState(awaitingStoreResult = true),
        subscriptionState = SubscriptionState.Canceled,
      )

    assertFalse(state.awaitingStoreResult)
    assertTrue(state.shouldClose)
  }

  @Test
  fun `subscription refresh keeps waiting when subscription is still active`() {
    val state =
      reduceCancelPlanFlowOnSubscriptionState(
        current = CancelPlanFlowState(awaitingStoreResult = true),
        subscriptionState = SubscriptionState.Active,
      )

    assertTrue(state.awaitingStoreResult)
    assertFalse(state.shouldClose)
  }

  @Test
  fun `consume close request clears only close flag`() {
    val state =
      consumeCancelPlanCloseRequest(
        CancelPlanFlowState(awaitingStoreResult = false, shouldClose = true)
      )

    assertEquals(CancelPlanFlowState(awaitingStoreResult = false, shouldClose = false), state)
  }

  @Test
  fun `consume error message clears only error message`() {
    val state =
      consumeCancelPlanErrorMessage(
        CancelPlanFlowState(
          awaitingStoreResult = true,
          shouldClose = false,
          errorMessage = OPEN_SUBSCRIPTION_MANAGEMENT_FAILURE_MESSAGE,
        )
      )

    assertEquals(
      CancelPlanFlowState(awaitingStoreResult = true, shouldClose = false, errorMessage = null),
      state,
    )
  }
}
