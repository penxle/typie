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

  private var lastKnown: Subscription? by mutableStateOf(null)
  private var clockTick by mutableStateOf(0L)

  val entitlement: Entitlement by derivedStateOf {
    @Suppress("UNUSED_EXPRESSION") clockTick
    when (val raw = query.state) {
      QueryState.Loading,
      is QueryState.Error ->
        if (lastKnown == null) Entitlement.Unknown
        else resolveEntitlement(lastKnown, Clock.System.now())
      is QueryState.Success -> {
        val me = raw.data.me
        if (me == null) {
          if (lastKnown == null) Entitlement.Unknown
          else resolveEntitlement(lastKnown, Clock.System.now())
        } else {
          resolveEntitlement(me.subscription?.toSubscription(), Clock.System.now())
        }
      }
    }
  }

  val subscription: Subscription? by derivedStateOf {
    when (val raw = query.state) {
      is QueryState.Success -> {
        val me = raw.data.me
        if (me == null) lastKnown else me.subscription?.toSubscription()
      }
      else -> lastKnown
    }
  }

  init {
    scope.launch {
      snapshotFlow {
        (query.state as? QueryState.Success)?.data?.me?.subscription?.toSubscription()
      }
        .collect { subscription -> if (subscription != null) lastKnown = subscription }
    }

    scope.launch {
      appLifecycleService.snapshot
        .map { it.foregroundGeneration }
        .distinctUntilChanged()
        .drop(1)
        .collect { query.refetch() }
    }

    scope.launch {
      // 만료일이 아닌 유효 판정 마감(유예 중이면 유예 상한)에 예약한다 — 만료일 틱만으로는
      // 유예 상한 도달 시 재평가가 없어 오프라인 유예 권한이 무기한 유지된다.
      snapshotFlow { subscription?.let(::entitlementDeadline) }
        .distinctUntilChanged()
        .collectLatest { deadline ->
          if (deadline == null) return@collectLatest
          while (true) {
            val remaining = deadline - Clock.System.now()
            if (!remaining.isPositive()) break
            delay(remaining)
          }
          // 판정 마감 도달: 서버 판정 우선(갱신 결제가 반영됐으면 새 마감으로 이 collect가 재시작됨),
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
    expiresAt = expiresAt,
    planId = plan.id,
    planName = plan.name,
    fee = plan.fee,
    availability = plan.availability,
  )
}

suspend fun SubscriptionService.gate(sheet: Sheet, action: GatedAction): Boolean {
  if (entitlement !is Entitlement.Expired) return true

  sheet.presentSubscribeSheet()
  return false
}
