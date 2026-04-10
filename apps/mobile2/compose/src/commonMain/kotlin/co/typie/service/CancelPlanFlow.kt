package co.typie.service

data class CancelPlanFlowState(
  val awaitingStoreResult: Boolean = false,
  val shouldClose: Boolean = false,
  val errorMessage: String? = null,
)

const val OPEN_SUBSCRIPTION_MANAGEMENT_FAILURE_MESSAGE = "스토어를 열 수 없어요. 잠시 후 다시 시도해주세요."

fun reduceCancelPlanFlowOnManagementResult(
  current: CancelPlanFlowState,
  result: SubscriptionManagementResult,
): CancelPlanFlowState {
  return when (result) {
    SubscriptionManagementResult.FailedToOpen ->
      current.copy(
        awaitingStoreResult = false,
        shouldClose = false,
        errorMessage = OPEN_SUBSCRIPTION_MANAGEMENT_FAILURE_MESSAGE,
      )
    SubscriptionManagementResult.AwaitingExternalResult ->
      current.copy(awaitingStoreResult = true, shouldClose = false, errorMessage = null)
    SubscriptionManagementResult.CompletedLocally ->
      current.copy(awaitingStoreResult = false, shouldClose = true, errorMessage = null)
  }
}

fun reduceCancelPlanFlowOnSubscriptionState(
  current: CancelPlanFlowState,
  subscriptionState: SubscriptionState?,
): CancelPlanFlowState {
  return if (
    shouldCloseCancelPlanAfterStoreReturn(current.awaitingStoreResult, subscriptionState)
  ) {
    current.copy(awaitingStoreResult = false, shouldClose = true, errorMessage = null)
  } else {
    current
  }
}

fun consumeCancelPlanCloseRequest(current: CancelPlanFlowState): CancelPlanFlowState {
  return current.copy(shouldClose = false)
}

fun consumeCancelPlanErrorMessage(current: CancelPlanFlowState): CancelPlanFlowState {
  return current.copy(errorMessage = null)
}
