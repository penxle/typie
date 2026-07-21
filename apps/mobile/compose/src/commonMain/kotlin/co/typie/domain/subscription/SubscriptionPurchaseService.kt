package co.typie.domain.subscription

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.typie.graphql.Apollo
import co.typie.graphql.SubscriptionPurchaseService_Query
import co.typie.graphql.SubscriptionPurchaseService_SubscribeOrChangePlanWithInAppPurchase_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.InAppPurchaseStore
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscribeOrChangePlanWithInAppPurchaseInput
import co.typie.platform.ActivityContext
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.platform.PurchaseEvent
import co.typie.platform.PurchaseProduct
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy
import kotlin.coroutines.cancellation.CancellationException
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull

object SubscriptionPurchaseService {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
  private val productsMutex = Mutex()

  var products by mutableStateOf<List<PurchaseProduct>>(emptyList())
    private set

  var productsUnavailable by mutableStateOf(false)
    private set

  val monthlyProduct: PurchaseProduct? by derivedStateOf {
    products.firstOrNull { it.planId == "pl0fl1map" }
  }

  val yearlyProduct: PurchaseProduct? by derivedStateOf {
    products.firstOrNull { it.planId == "pl0fl1yap" }
  }

  private val _completions = MutableSharedFlow<Unit>()
  val completions: SharedFlow<Unit> = _completions

  private val _failures = MutableSharedFlow<PurchaseFailure>()
  val failures: SharedFlow<PurchaseFailure> = _failures

  var registrationGeneration by mutableStateOf(0L)
    private set

  private var launched = false

  fun launch() {
    if (launched) return
    launched = true

    scope.launch(start = CoroutineStart.UNDISPATCHED) {
      PlatformModule.purchaseService.events.collect { handlePurchaseEvent(it) }
    }
  }

  suspend fun ensureProductsLoaded() {
    productsMutex.withLock {
      if (products.isNotEmpty()) {
        return
      }

      try {
        products =
          PlatformModule.purchaseService.queryProducts(storeProductIds(PlatformModule.platform))
        productsUnavailable = products.isEmpty()
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        productsUnavailable = true
      }
    }
  }

  context(_: ActivityContext)
  suspend fun purchase(product: PurchaseProduct): Boolean {
    val me =
      try {
        Apollo.query(SubscriptionPurchaseService_Query())
          .fetchPolicy(FetchPolicy.NetworkOnly)
          .execute()
          .dataOrThrow()
          .me
      } catch (e: CancellationException) {
        throw e
      } catch (_: Exception) {
        _failures.emit(PurchaseFailure.PreflightFailed)
        return false
      }

    // 서버는 만료일과 무관하게 비-EXPIRED 상태의 타 채널 구독이 있으면 subscription_already_exists 로 거부한다.
    // 앱 내 결제 후 과금됐는데 등록이 거부되는 것을 막기 위해, 캐시가 아닌 최신 서버 응답으로 동일 기준을 선차단한다.
    // (me.subscription 은 비-EXPIRED 구독만 노출하므로 non-null 이면 서버가 거부한다)
    val current = me.subscription
    if (
      current != null &&
        current.plan.availability != PlanAvailability.IN_APP_PURCHASE &&
        current.plan.availability != PlanAvailability.TRIAL
    ) {
      _failures.emit(PurchaseFailure.ConflictBeforePurchase)
      return false
    }

    return PlatformModule.purchaseService.purchase(product = product, accountId = me.uuid)
  }

  suspend fun awaitRegistration(sinceGeneration: Long) {
    withTimeoutOrNull(15.seconds) {
      snapshotFlow { registrationGeneration }.first { it > sinceGeneration }
    }
  }

  private suspend fun handlePurchaseEvent(event: PurchaseEvent) {
    try {
      val previousSubscriptionId = SubscriptionService.subscription?.id

      val response =
        Apollo.executeMutation(
          SubscriptionPurchaseService_SubscribeOrChangePlanWithInAppPurchase_Mutation(
            input =
              SubscribeOrChangePlanWithInAppPurchaseInput(
                data = event.subscriptionId,
                store =
                  when (PlatformModule.platform) {
                    Platform.Android -> InAppPurchaseStore.GOOGLE_PLAY
                    Platform.iOS -> InAppPurchaseStore.APP_STORE
                    else ->
                      throw IllegalArgumentException(
                        "Unsupported platform: ${PlatformModule.platform}"
                      )
                  },
              )
          )
        )

      PlatformModule.purchaseService.finishTransaction(event.subscriptionId)
      SubscriptionService.refresh()

      if (
        isNewSubscription(
          previousSubscriptionId,
          response.subscribeOrChangePlanWithInAppPurchase.id,
        )
      ) {
        _completions.emit(Unit)
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: TypieError) {
      // 스토어 과금은 이미 발생했고 트랜잭션은 미완료로 남아 다음 앱 실행 시 자동 재시도된다
      // (iOS 는 pending 재전송, Android 는 recoverPurchases). 여기서는 사유만 사용자에게 알린다.
      when (e.code) {
        "subscription_already_exists" -> _failures.emit(PurchaseFailure.ConflictAfterPurchase)
        "in_app_purchase_account_mismatch" -> _failures.emit(PurchaseFailure.AccountMismatch)
      }
    } catch (_: Exception) {
      // best effort
    } finally {
      registrationGeneration += 1
    }
  }
}

enum class PurchaseFailure {
  ConflictBeforePurchase,
  ConflictAfterPurchase,
  AccountMismatch,
  PreflightFailed,
}

internal fun storeProductIds(platform: Platform): List<String> =
  when (platform) {
    Platform.Android -> listOf("plan.full")
    else -> listOf("pl0fl1map", "pl0fl1yap")
  }

internal fun isNewSubscription(
  previousSubscriptionId: String?,
  newSubscriptionId: String,
): Boolean = previousSubscriptionId != newSubscriptionId
