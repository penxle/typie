package co.typie.screen.subscription

import co.typie.di.Platform
import co.typie.graphql.QueryState
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchaseProduct
import co.typie.platform.PurchaseService
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow
import org.koin.core.annotation.Single

data class SubscriptionSummary(
  val hasSubscription: Boolean,
  val subscriptionName: String,
)

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

@Single
class SubscriptionService(
  private val platform: Platform,
  private val purchaseService: PurchaseService,
  private val subscriptionDevSandbox: SubscriptionDevSandbox,
  private val subscriptionSync: SubscriptionSync,
) {
  val usesSandbox: Boolean
    get() = platform == Platform.Desktop && subscriptionDevSandbox.usesSandbox

  val purchaseEvents: Flow<PurchaseEvent>
    get() = if (usesSandbox) {
      emptyFlow()
    } else {
      purchaseService.events
    }

  fun hasQueryError(state: QueryState<*>): Boolean {
    return !usesSandbox && state is QueryState.Error
  }

  fun isQueryLoading(state: QueryState<*>): Boolean {
    return !usesSandbox && state !is QueryState.Success
  }

  fun currentSubscription(remoteSubscription: SubscriptionSnapshot?): SubscriptionSnapshot? {
    return if (usesSandbox) {
      subscriptionDevSandbox.currentSubscription
    } else {
      remoteSubscription
    }
  }

  fun hasSubscription(remoteSubscription: SubscriptionSnapshot?): Boolean {
    return currentSubscription(remoteSubscription) != null
  }

  fun summary(remoteSubscription: SubscriptionSnapshot?): SubscriptionSummary {
    val data = currentSubscription(remoteSubscription)
    return SubscriptionSummary(
      hasSubscription = data != null,
      subscriptionName = data?.planName ?: "타이피 BASIC ACCESS",
    )
  }

  fun canStartTrial(remoteCanStartTrial: Boolean): Boolean {
    return if (usesSandbox) {
      subscriptionDevSandbox.canStartTrial
    } else {
      remoteCanStartTrial
    }
  }

  suspend fun queryProducts(): Map<PurchasePlanInterval, PurchaseProduct> {
    return purchaseService.queryProducts()
  }

  suspend fun startTrial(remoteAction: suspend () -> Unit): SubscriptionCelebration {
    if (usesSandbox) {
      subscriptionDevSandbox.startTrial()
    } else {
      remoteAction()
    }

    subscriptionSync.notifyChanged()

    return trialCelebration()
  }

  suspend fun purchase(
    product: PurchaseProduct,
    accountId: String,
  ): PurchaseStartResult {
    if (usesSandbox) {
      subscriptionDevSandbox.purchase(product.interval)
      subscriptionSync.notifyChanged()

      return PurchaseStartResult(
        started = true,
        celebration = purchaseCelebration(),
      )
    }

    return PurchaseStartResult(
      started = purchaseService.purchase(
        product = product,
        accountId = accountId,
      ),
    )
  }

  suspend fun openSubscriptionManagement(): SubscriptionManagementResult {
    if (usesSandbox) {
      subscriptionDevSandbox.scheduleCancel()
      subscriptionSync.notifyChanged()
      return SubscriptionManagementResult.CompletedLocally
    }

    return if (purchaseService.openSubscriptionManagement()) {
      SubscriptionManagementResult.AwaitingExternalResult
    } else {
      SubscriptionManagementResult.FailedToOpen
    }
  }

  fun notifyChanged() {
    subscriptionSync.notifyChanged()
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
