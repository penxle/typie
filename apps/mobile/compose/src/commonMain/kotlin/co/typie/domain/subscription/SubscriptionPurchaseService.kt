package co.typie.domain.subscription

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.touchlab.kermit.Logger
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
import co.typie.platform.PurchaseReplacementMode
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
        Apollo.query(SubscriptionPurchaseService_Query(store = currentStore()))
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

    // 서버 등록이 쓰는 것과 같은 판정이다. 다만 advisory 다 — 스토어 결제와 서버 등록 사이에 원자성이 없어
    // 통과한 뒤에도 등록이 거절될 수 있다.
    if (!me.canEnrollInAppPurchase) {
      _failures.emit(PurchaseFailure.ConflictBeforePurchase)
      return false
    }

    // 같은 스토어의 IAP 구독을 보유한 채로 다른 플랜을 사면 플랜 변경이다 — 승계 파라미터 없이 시작하면
    // 독립 토큰이 되어 서버 등록에서 거절된다.
    val current = me.subscription
    val existingPurchaseToken =
      if (current?.plan?.availability == PlanAvailability.IN_APP_PURCHASE) {
        PlatformModule.purchaseService.currentSubscriptionPurchaseToken()
      } else {
        null
      }
    val replacementMode =
      if (current != null && existingPurchaseToken != null) {
        planReplacementMode(currentPlanId = current.plan.id, newPlanId = product.planId)
      } else {
        null
      }

    return PlatformModule.purchaseService.purchase(
      product = product,
      accountId = me.uuid,
      existingPurchaseToken = existingPurchaseToken.takeIf { replacementMode != null },
      replacementMode = replacementMode,
    )
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
                store = currentStore(),
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
        // 서버가 결제를 소유 증거의 계정에 이미 귀속시킨 뒤 내려주는 코드다 — 이 세션에서 재시도해도 결과가
        // 바뀌지 않으므로 트랜잭션을 종료해 재시도 루프를 끊는다. 결제는 소유 계정에 반영돼 있어 유실되지 않는다.
        "in_app_purchase_account_mismatch" -> {
          PlatformModule.purchaseService.finishTransaction(event.subscriptionId)
          _failures.emit(PurchaseFailure.AccountMismatch)
        }
        // 스토어가 종료를 확정한 구매다 — 재시도해도 결과가 바뀌지 않으므로 트랜잭션을 종료해
        // 앱 실행마다 반복되는 재등록 루프를 끊는다. 이미 만료된 구매라 종료로 유실되는 권한도 없다.
        "in_app_purchase_expired" -> {
          Logger.w { "in-app purchase already expired: finishing transaction" }
          PlatformModule.purchaseService.finishTransaction(event.subscriptionId)
        }
        // 등록 경합·불변식 위반이다. 재시도로 풀리거나 사람이 정리해야 하므로 사용자에게 알리지 않는다.
        "in_app_purchase_registration_conflict" ->
          Logger.w { "in-app purchase registration conflict: retrying on next launch" }
        else -> Logger.w { "in-app purchase registration rejected: ${e.code}" }
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

private fun currentStore(): InAppPurchaseStore =
  when (PlatformModule.platform) {
    Platform.Android -> InAppPurchaseStore.GOOGLE_PLAY
    Platform.iOS -> InAppPurchaseStore.APP_STORE
    else -> throw IllegalArgumentException("Unsupported platform: ${PlatformModule.platform}")
  }

// 서버 플랜 ID 는 대문자(PL0FL1MAP), 스토어 base plan ID 는 소문자(pl0fl1map)로 같은 플랜을 가리킨다.
private fun planRank(planId: String): Int =
  when (planId.lowercase()) {
    "pl0fl1map" -> 0
    "pl0fl1yap" -> 1
    else -> -1
  }

/** 업그레이드(월간 → 연간)는 잔여 가치를 시간으로 환산해 즉시 발효하고, 다운그레이드(연간 → 월간)는 기간이 끝난 뒤 발효한다. 같은 플랜이면 변경이 아니다. */
internal fun planReplacementMode(
  currentPlanId: String,
  newPlanId: String,
): PurchaseReplacementMode? =
  when {
    currentPlanId.equals(newPlanId, ignoreCase = true) -> null
    planRank(newPlanId) > planRank(currentPlanId) ->
      PurchaseReplacementMode.UPGRADE_WITH_TIME_PRORATION
    else -> PurchaseReplacementMode.DEFERRED
  }

internal fun isNewSubscription(
  previousSubscriptionId: String?,
  newSubscriptionId: String,
): Boolean = previousSubscriptionId != newSubscriptionId
