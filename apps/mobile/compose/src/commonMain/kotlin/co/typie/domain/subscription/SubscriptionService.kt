package co.typie.domain.subscription

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import co.typie.domain.auth.AuthService
import co.typie.domain.auth.AuthState
import co.typie.graphql.Apollo
import co.typie.graphql.QueryState
import co.typie.graphql.SubscriptionService_Query
import co.typie.graphql.watchQuery
import co.typie.platform.appLifecycleService
import co.typie.storage.Preference
import co.typie.ui.component.sheet.Sheet
import kotlin.time.Clock
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch

object SubscriptionService {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

  private val query =
    Apollo.watchQuery(scope = scope, skip = { AuthService.state !is AuthState.Authenticated }) {
      SubscriptionService_Query()
    }

  private var clockTick by mutableStateOf(0L)

  // me 응답 전(콜드 스타트·오프라인)에도 캐시가 세션 유저로 필터되도록 인증 시점 userId에서 파생한다.
  private val sessionUserId: String?
    get() = (AuthService.state as? AuthState.Authenticated)?.tokens?.userId

  private val me: SubscriptionService_Query.Me?
    get() = (query.state as? QueryState.Success)?.data?.me

  private val cache: EntitlementCache?
    get() = entitlementCacheFor(Preference.entitlementCache, sessionUserId)

  val entitlement: Entitlement by derivedStateOf {
    @Suppress("UNUSED_EXPRESSION") clockTick
    val me = me
    // 서버 응답이 있으면 그 값이 권한이다. 응답이 없는 동안(최초 로딩·오프라인)만 캐시를 유효 창 안에서 쓴다.
    if (me != null) resolveEntitlement(me.entitled, me.subscription?.toSubscription())
    else resolveEntitlement(cache, Clock.System.now())
  }

  val subscription: Subscription? by derivedStateOf {
    val me = me
    if (me != null) me.subscription?.toSubscription() else cache?.subscription?.toSubscription()
  }

  init {
    scope.launch {
      // 데이터가 그대로여도 서버 응답마다 다시 쓴다 — checkedAt 은 "값이 바뀐 시각"이 아니라
      // "마지막으로 서버에 확인한 시각"이어야 유효 창이 온라인에서 조기 폐쇄되지 않는다.
      snapshotFlow { query.networkGeneration to me }
        .collect { (_, me) ->
          if (me == null) return@collect
          // 구독이 없는 응답(entitled = false)도 저장한다 — 회수를 관측한 뒤 오프라인이 되면
          // 과거 true 가 되살아난다.
          Preference.entitlementCache =
            entitlementCacheOf(
              userId = me.id,
              entitled = me.entitled,
              entitledUntil = me.entitledUntil,
              subscription = me.subscription?.toSubscription(),
              checkedAt = Clock.System.now(),
            )
        }
    }

    scope.launch {
      appLifecycleService.snapshot
        .map { it.foregroundGeneration }
        .distinctUntilChanged()
        .drop(1)
        .collect { query.refetch() }
    }

    scope.launch {
      // 캐시 유효 창의 끝에 예약한다. entitledUntil 이 없는 ACTIVE 류에도 72시간 상한 타이머가 항상
      // 걸려야, foreground 오프라인 앱이 상한을 지나도 재평가 없이 fail-open 으로 남지 않는다.
      snapshotFlow { cache?.let(::entitlementCacheDeadline) }
        .distinctUntilChanged()
        .collectLatest { deadline ->
          if (deadline == null) return@collectLatest
          while (true) {
            val remaining = deadline - Clock.System.now()
            if (!remaining.isPositive()) break
            delay(remaining)
          }
          // 마감 도달: 서버 판정 우선(응답이 오면 새 마감으로 이 collect가 재시작됨),
          // 확인 불가(오프라인)면 clockTick 재평가로 비관 강등된다.
          query.refetch()
          clockTick += 1
        }
    }
  }

  fun refresh() {
    query.refetch()
  }

  private val gateRequestChannel = Channel<GatedAction>(Channel.CONFLATED)
  val gateRequests: Flow<GatedAction> = gateRequestChannel.receiveAsFlow()

  fun requestSubscribeSheet(action: GatedAction = GatedAction.Generic) {
    gateRequestChannel.trySend(action)
  }

  fun drainGateRequests() {
    while (gateRequestChannel.tryReceive().isSuccess) {}
  }
}

private fun SubscriptionService_Query.Subscription.toSubscription(): Subscription {
  return Subscription(
    id = id,
    state = state,
    startsAt = startsAt,
    currentPeriodEndsAt = currentPeriodEndsAt,
    planId = plan.id,
    planName = plan.name,
    fee = plan.fee,
    availability = plan.availability,
  )
}

suspend fun SubscriptionService.gate(sheet: Sheet, action: GatedAction): Boolean {
  if (entitlement.grantsAccess()) return true

  sheet.presentSubscribeSheet()
  return false
}
