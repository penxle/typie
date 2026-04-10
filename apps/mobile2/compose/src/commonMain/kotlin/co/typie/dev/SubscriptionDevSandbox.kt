package co.typie.dev

import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.platform.PurchasePlanInterval
import co.typie.service.FULL_ACCESS_MONTHLY_PLAN_ID
import co.typie.service.FULL_ACCESS_YEARLY_PLAN_ID
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionSnapshot
import co.typie.service.SubscriptionState
import kotlinx.datetime.LocalDate
import kotlinx.datetime.TimeZone
import kotlinx.datetime.atStartOfDayIn
import kotlinx.datetime.toLocalDateTime
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlin.time.Clock
import kotlin.time.Instant

enum class SubscriptionDevScenario(
  val label: String,
) {
  RemoteData("실제 구독 상태 사용"),
  NoSubscription("없음"),
  TrialExpired("무료 체험 종료"),
  Trial("무료 체험 중"),
  Monthly("월간 이용 중"),
  Yearly("연간 이용 중"),
  CancelScheduled("해지 예정"),
  BillingKey("웹 결제"),
  Manual("수동 이용권"),
}

object SubscriptionDevSandbox {
  private val _scenario = MutableStateFlow(SubscriptionDevScenario.RemoteData)
  val scenario: StateFlow<SubscriptionDevScenario> = _scenario

  val enabled: Boolean
    get() = PlatformModule.platform == Platform.Desktop

  val usesSandbox: Boolean
    get() = enabled && _scenario.value != SubscriptionDevScenario.RemoteData

  val currentSubscription: SubscriptionSnapshot?
    get() = subscriptionDevSubscription(_scenario.value)

  val canStartTrial: Boolean
    get() = subscriptionDevCanStartTrial(_scenario.value)

  fun select(next: SubscriptionDevScenario) {
    if (!enabled) return
    _scenario.value = next
  }

  fun startTrial() {
    select(SubscriptionDevScenario.Trial)
  }

  fun purchase(interval: PurchasePlanInterval) {
    select(
      when (interval) {
        PurchasePlanInterval.Monthly -> SubscriptionDevScenario.Monthly
        PurchasePlanInterval.Yearly -> SubscriptionDevScenario.Yearly
      },
    )
  }

  fun scheduleCancel() {
    select(SubscriptionDevScenario.CancelScheduled)
  }
}

fun subscriptionDevCanStartTrial(scenario: SubscriptionDevScenario): Boolean {
  return scenario == SubscriptionDevScenario.NoSubscription
}

fun subscriptionDevSubscription(
  scenario: SubscriptionDevScenario,
  now: Instant = Clock.System.now(),
): SubscriptionSnapshot? {
  val timeZone = TimeZone.currentSystemDefault()
  val today = now.toLocalDateTime(timeZone).date
  val startsAt = today.atStartOfDayIn(timeZone)
  val monthlyExpiresAt = relativeDate(today, 31).atStartOfDayIn(timeZone)

  return when (scenario) {
    SubscriptionDevScenario.RemoteData -> null
    SubscriptionDevScenario.NoSubscription -> null
    SubscriptionDevScenario.TrialExpired -> null
    SubscriptionDevScenario.Trial -> SubscriptionSnapshot(
      id = "subscription-trial",
      state = SubscriptionState.Active,
      startsAt = startsAt,
      expiresAt = relativeDate(today, 14).atStartOfDayIn(timeZone),
      planId = "trial-plan",
      planName = "타이피 FULL ACCESS (체험)",
      fee = 0,
      availability = SubscriptionAvailability.Trial,
    )

    SubscriptionDevScenario.Monthly -> SubscriptionSnapshot(
      id = "subscription-monthly",
      state = SubscriptionState.Active,
      startsAt = startsAt,
      expiresAt = monthlyExpiresAt,
      planId = FULL_ACCESS_MONTHLY_PLAN_ID,
      planName = "타이피 FULL ACCESS",
      fee = 12_900,
      availability = SubscriptionAvailability.InAppPurchase,
    )

    SubscriptionDevScenario.Yearly -> SubscriptionSnapshot(
      id = "subscription-yearly",
      state = SubscriptionState.Active,
      startsAt = startsAt,
      expiresAt = relativeDate(today, 365).atStartOfDayIn(timeZone),
      planId = FULL_ACCESS_YEARLY_PLAN_ID,
      planName = "타이피 FULL ACCESS",
      fee = 129_000,
      availability = SubscriptionAvailability.InAppPurchase,
    )

    SubscriptionDevScenario.CancelScheduled -> SubscriptionSnapshot(
      id = "subscription-cancel-scheduled",
      state = SubscriptionState.Canceled,
      startsAt = startsAt,
      expiresAt = monthlyExpiresAt,
      planId = FULL_ACCESS_MONTHLY_PLAN_ID,
      planName = "타이피 FULL ACCESS",
      fee = 12_900,
      availability = SubscriptionAvailability.InAppPurchase,
    )

    SubscriptionDevScenario.BillingKey -> SubscriptionSnapshot(
      id = "subscription-billing-key",
      state = SubscriptionState.Active,
      startsAt = startsAt,
      expiresAt = monthlyExpiresAt,
      planId = "billing-key-plan",
      planName = "타이피 FULL ACCESS",
      fee = 12_900,
      availability = SubscriptionAvailability.BillingKey,
    )

    SubscriptionDevScenario.Manual -> SubscriptionSnapshot(
      id = "subscription-manual",
      state = SubscriptionState.Active,
      startsAt = startsAt,
      expiresAt = monthlyExpiresAt,
      planId = "manual-plan",
      planName = "타이피 FULL ACCESS",
      fee = 12_900,
      availability = SubscriptionAvailability.Manual,
    )
  }
}

private fun relativeDate(
  date: LocalDate,
  days: Int,
): LocalDate {
  return LocalDate.fromEpochDays(date.toEpochDays() + days)
}
