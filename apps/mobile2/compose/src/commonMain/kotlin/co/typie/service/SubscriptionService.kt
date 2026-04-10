package co.typie.service

import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.graphql.QueryState
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchaseProduct
import co.typie.dev.SubscriptionDevSandbox
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow

data class SubscriptionCelebration(
  val title: String,
  val message: String,
)

data class PurchaseStartResult(
  val started: Boolean,
  val celebration: SubscriptionCelebration? = null,
)

enum class SubscriptionManagementResult {
  FailedToOpen,
  AwaitingExternalResult,
  CompletedLocally,
}

object SubscriptionService {
  val usesSandbox: Boolean
    get() = PlatformModule.platform == Platform.Desktop && SubscriptionDevSandbox.usesSandbox

  val purchaseEvents: Flow<PurchaseEvent>
    get() = if (usesSandbox) {
      emptyFlow()
    } else {
      PlatformModule.purchaseService.events
    }

  fun hasQueryError(state: QueryState<*>): Boolean {
    return !usesSandbox && state is QueryState.Error
  }

  fun isQueryLoading(state: QueryState<*>): Boolean {
    return !usesSandbox && state !is QueryState.Success
  }

  fun canStartTrial(remoteCanStartTrial: Boolean): Boolean {
    return if (usesSandbox) {
      SubscriptionDevSandbox.canStartTrial
    } else {
      remoteCanStartTrial
    }
  }

  suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct> {
    return PlatformModule.purchaseService.queryProducts()
  }

  suspend fun startTrial(remoteAction: suspend () -> Unit): SubscriptionCelebration {
    if (usesSandbox) {
      SubscriptionDevSandbox.startTrial()
    } else {
      remoteAction()
    }

    return trialCelebration()
  }

  suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
  ): PurchaseStartResult {
    if (usesSandbox) {
      SubscriptionDevSandbox.purchase(product.interval)

      return PurchaseStartResult(
        started = true,
        celebration = purchaseCelebration(),
      )
    }

    return PurchaseStartResult(
      started = PlatformModule.purchaseService.purchase(
        product = product,
        accountId = accountId,
      ),
    )
  }

  suspend fun openSubscriptionManagement(): SubscriptionManagementResult {
    if (usesSandbox) {
      SubscriptionDevSandbox.scheduleCancel()
      return SubscriptionManagementResult.CompletedLocally
    }

    return if (PlatformModule.purchaseService.openSubscriptionManagement()) {
      SubscriptionManagementResult.AwaitingExternalResult
    } else {
      SubscriptionManagementResult.FailedToOpen
    }
  }
}

internal fun trialCelebration(): SubscriptionCelebration {
  return SubscriptionCelebration(
    title = "무료 체험이 시작됐어요!",
    message = "2주간 타이피의 모든 기능을 자유롭게 이용해보세요.",
  )
}

internal fun purchaseCelebration(): SubscriptionCelebration {
  return SubscriptionCelebration(
    title = "구독이 시작됐어요!",
    message = "타이피의 모든 기능을 자유롭게 이용해보세요.",
  )
}
