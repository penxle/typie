package co.typie.domain.subscription

import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant
import kotlinx.serialization.Serializable

// 마지막 서버 확인 이후 오프라인에서 last-known 을 신뢰하는 상한.
// 무기한 캐시는 환불 직후 영구 권한이 되고, 즉시 폐기는 짧은 네트워크 장애를 fail-closed 로 만든다.
val ENTITLEMENT_CACHE_MAX_AGE = 72.hours

sealed interface Entitlement {
  data object Unknown : Entitlement

  data class Active(val subscription: Subscription?, val inGracePeriod: Boolean) : Entitlement

  data object Expired : Entitlement
}

/**
 * 표시·구성용 구독 스냅샷. 도메인 [Subscription] 과 Apollo enum 은 kotlinx-serialization 대상이 아니라 영속 캐시에 그대로 넣을 수
 * 없어, enum 은 raw string 으로 시각은 epoch millis 로 눕힌다.
 */
@Serializable
data class EntitlementSubscriptionSnapshot(
  val id: String,
  val state: String,
  val startsAtEpochMs: Long,
  val currentPeriodEndsAtEpochMs: Long,
  val planId: String,
  val planName: String,
  val fee: Int,
  val availability: String,
)

/** 세션 유저에 귀속된 last-known 권한. 권한 판정은 [entitled] 만 쓰고 스냅샷은 표시·구성 전용이다. */
@Serializable
data class EntitlementCache(
  val userId: String,
  val entitled: Boolean,
  val entitledUntilEpochMs: Long?,
  val checkedAtEpochMs: Long,
  val subscription: EntitlementSubscriptionSnapshot?,
)

fun Subscription.toSnapshot(): EntitlementSubscriptionSnapshot =
  EntitlementSubscriptionSnapshot(
    id = id,
    state = state.rawValue,
    startsAtEpochMs = startsAt.toEpochMilliseconds(),
    currentPeriodEndsAtEpochMs = currentPeriodEndsAt.toEpochMilliseconds(),
    planId = planId,
    planName = planName,
    fee = fee,
    availability = availability.rawValue,
  )

fun EntitlementSubscriptionSnapshot.toSubscription(): Subscription =
  Subscription(
    id = id,
    state = SubscriptionState.safeValueOf(state),
    startsAt = Instant.fromEpochMilliseconds(startsAtEpochMs),
    currentPeriodEndsAt = Instant.fromEpochMilliseconds(currentPeriodEndsAtEpochMs),
    planId = planId,
    planName = planName,
    fee = fee,
    availability = PlanAvailability.safeValueOf(availability),
  )

fun entitlementCacheOf(
  userId: String,
  entitled: Boolean,
  entitledUntil: Instant?,
  subscription: Subscription?,
  checkedAt: Instant,
): EntitlementCache =
  EntitlementCache(
    userId = userId,
    entitled = entitled,
    entitledUntilEpochMs = entitledUntil?.toEpochMilliseconds(),
    checkedAtEpochMs = checkedAt.toEpochMilliseconds(),
    subscription = subscription?.toSnapshot(),
  )

val EntitlementCache.checkedAt: Instant
  get() = Instant.fromEpochMilliseconds(checkedAtEpochMs)

val EntitlementCache.entitledUntil: Instant?
  get() = entitledUntilEpochMs?.let(Instant::fromEpochMilliseconds)

/** 캐시 유효 창의 끝. `entitledUntil` 이 없는 ACTIVE 류에도 상한이 존재해야, 오프라인 앱이 상한을 지나도록 재평가 없이 남지 않는다. */
fun entitlementCacheDeadline(cache: EntitlementCache): Instant =
  minOf(cache.entitledUntil ?: Instant.DISTANT_FUTURE, cache.checkedAt + ENTITLEMENT_CACHE_MAX_AGE)

/** 서버 응답 그대로의 판정. 시각 비교는 서버가 이미 끝냈으므로 여기서 다시 하지 않는다. */
fun resolveEntitlement(entitled: Boolean, subscription: Subscription?): Entitlement =
  if (entitled) {
    Entitlement.Active(
      subscription = subscription,
      inGracePeriod = subscription?.state == SubscriptionState.IN_GRACE_PERIOD,
    )
  } else {
    Entitlement.Expired
  }

/**
 * 서버 응답이 없을 때의 판정. 유효 창 안에서만 last-known 을 쓰고, 창 밖은 `Expired` 로 강등한다. `Unknown` 은 캐시도 서버 값도 없는 최초 순간
 * 전용이다.
 */
fun resolveEntitlement(cache: EntitlementCache?, now: Instant): Entitlement =
  when {
    cache == null -> Entitlement.Unknown
    now >= entitlementCacheDeadline(cache) -> Entitlement.Expired
    else -> resolveEntitlement(cache.entitled, cache.subscription?.toSubscription())
  }

/**
 * 캐시는 세션 토큰에 귀속된다. 토큰이 그대로면 유지해야 매 시작마다 같은 토큰으로 도는 bootstrap `renew()` 가 오프라인 복원을 지우지 않는다. 반대로 앞선
 * 토큰이 없는데 캐시만 남아 있는 상태(기기 백업 복원 — prefs 는 평문 백업되고 vault 는 복원되지 않는다)는 세션에 귀속되지 않은 값이라 폐기한다.
 */
fun shouldDiscardEntitlementCache(
  previousSessionToken: String?,
  nextSessionToken: String,
): Boolean = previousSessionToken != nextSessionToken

/** 다른 유저의 캐시는 없는 것과 같다 — 그대로 쓰면 A 의 권한이 B 에게 전이된다. */
fun entitlementCacheFor(cache: EntitlementCache?, sessionUserId: String?): EntitlementCache? =
  cache?.takeIf {
    sessionUserId == null || it.userId == sessionUserId
  }

/**
 * 접근 게이트의 단일 판정. `Expired` 만 차단하고 `Unknown`(최초 순간)·유효 창 안의 오프라인은 통과시킨다 — 서버 확인 전에 잠그면 첫 실행·짧은 네트워크
 * 장애가 fail-closed 가 된다.
 */
fun Entitlement.grantsAccess(): Boolean = this !is Entitlement.Expired
