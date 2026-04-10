package co.typie.screen.subscription

import co.typie.dev.SubscriptionDevSandbox
import co.typie.dev.SubscriptionDevScenario
import co.typie.dev.subscriptionDevCanStartTrial
import co.typie.dev.subscriptionDevSubscription
import co.typie.di.Platform
import co.typie.graphql.QueryState
import co.typie.platform.PurchasePlanInterval
import co.typie.service.FULL_ACCESS_MONTHLY_PLAN_ID
import co.typie.service.FULL_ACCESS_YEARLY_PLAN_ID
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionSnapshot
import co.typie.service.SubscriptionState
import co.typie.service.SubscriptionSummary
import co.typie.service.hasSubscriptionOrNull
import co.typie.service.isCurrentFullPlan
import co.typie.service.shouldAutoCloseCurrentPlan
import co.typie.service.shouldShowPurchaseCelebration
import co.typie.service.subscriptionSummaryOrNull
import kotlinx.datetime.LocalDate
import kotlinx.datetime.TimeZone
import kotlinx.datetime.atStartOfDayIn
import kotlinx.datetime.toLocalDateTime
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Instant

class SubscriptionModelsTest {
  @Test
  fun `desktop subscription sandbox defaults to real data mode`() {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop)

    assertEquals(SubscriptionDevScenario.RemoteData, sandbox.scenario.value)
    assertEquals(false, sandbox.usesSandbox)
  }

  @Test
  fun `desktop subscription sandbox transitions through trial purchase and cancel states`() {
    val sandbox = SubscriptionDevSandbox(Platform.Desktop)

    sandbox.startTrial()
    assertEquals(SubscriptionDevScenario.Trial, sandbox.scenario.value)

    sandbox.purchase(PurchasePlanInterval.Yearly)
    assertEquals(SubscriptionDevScenario.Yearly, sandbox.scenario.value)

    sandbox.scheduleCancel()
    assertEquals(SubscriptionDevScenario.CancelScheduled, sandbox.scenario.value)
  }

  @Test
  fun `subscriptionDevCanStartTrial returns true when no subscription exists`() {
    assertEquals(true, subscriptionDevCanStartTrial(SubscriptionDevScenario.NoSubscription))
  }

  @Test
  fun `subscriptionDevSubscription returns canceled in app purchase plan for cancel scheduled scenario`() {
    val data = subscriptionDevSubscription(SubscriptionDevScenario.CancelScheduled)

    assertEquals(SubscriptionState.Canceled, data?.state)
    assertEquals(SubscriptionAvailability.InAppPurchase, data?.availability)
    assertEquals(FULL_ACCESS_MONTHLY_PLAN_ID, data?.planId)
  }

  @Test
  fun `subscriptionDevSubscription uses relative dates from current date`() {
    val now = Instant.parse("2026-04-01T15:45:00Z")
    val timeZone = TimeZone.currentSystemDefault()
    val today = now.toLocalDateTime(timeZone).date
    val todayStartsAt = today.atStartOfDayIn(timeZone)

    val trial = subscriptionDevSubscription(SubscriptionDevScenario.Trial, now = now)
    val monthly = subscriptionDevSubscription(SubscriptionDevScenario.Monthly, now = now)
    val yearly = subscriptionDevSubscription(SubscriptionDevScenario.Yearly, now = now)
    val cancelScheduled = subscriptionDevSubscription(SubscriptionDevScenario.CancelScheduled, now = now)

    assertEquals(todayStartsAt, trial?.startsAt)
    assertEquals(relativeDate(today, 14).atStartOfDayIn(timeZone), trial?.expiresAt)
    assertEquals(todayStartsAt, monthly?.startsAt)
    assertEquals(relativeDate(today, 31).atStartOfDayIn(timeZone), monthly?.expiresAt)
    assertEquals(todayStartsAt, yearly?.startsAt)
    assertEquals(relativeDate(today, 365).atStartOfDayIn(timeZone), yearly?.expiresAt)
    assertEquals(todayStartsAt, cancelScheduled?.startsAt)
    assertEquals(relativeDate(today, 31).atStartOfDayIn(timeZone), cancelScheduled?.expiresAt)
  }

  @Test
  fun `subscriptionProductState returns loading while products are still being queried`() {
    assertEquals(
      SubscriptionProductState.Loading,
      subscriptionProductState(product = null, productsLoaded = false),
    )
  }

  @Test
  fun `subscriptionProductState returns unavailable when query finished without a matching product`() {
    assertEquals(
      SubscriptionProductState.Unavailable,
      subscriptionProductState(product = null, productsLoaded = true),
    )
  }

  @Test
  fun `isCurrentFullPlan matches current interval plan id`() {
    assertEquals(
      true,
      isCurrentFullPlan(
        currentPlanId = FULL_ACCESS_MONTHLY_PLAN_ID,
        interval = PurchasePlanInterval.Monthly,
      ),
    )
  }

  @Test
  fun `shouldShowPurchaseCelebration returns false when subscription and plan are unchanged`() {
    assertEquals(
      false,
      shouldShowPurchaseCelebration(
        originalSubscriptionId = "subscription-1",
        originalPlanId = FULL_ACCESS_MONTHLY_PLAN_ID,
        updatedSubscriptionId = "subscription-1",
        updatedPlanId = FULL_ACCESS_MONTHLY_PLAN_ID,
      ),
    )
  }

  @Test
  fun `shouldShowPurchaseCelebration returns true when plan changes`() {
    assertEquals(
      true,
      shouldShowPurchaseCelebration(
        originalSubscriptionId = "subscription-1",
        originalPlanId = FULL_ACCESS_MONTHLY_PLAN_ID,
        updatedSubscriptionId = "subscription-1",
        updatedPlanId = FULL_ACCESS_YEARLY_PLAN_ID,
      ),
    )
  }

  @Test
  fun `subscriptionEntryDestination returns current plan when subscription exists`() {
    assertEquals(
      SubscriptionEntryDestination.CurrentPlan,
      subscriptionEntryDestination(hasSubscription = true),
    )
  }

  @Test
  fun `subscriptionEntryDestination returns enroll plan when subscription does not exist`() {
    assertEquals(
      SubscriptionEntryDestination.EnrollPlan,
      subscriptionEntryDestination(hasSubscription = false),
    )
  }

  @Test
  fun `shouldAutoCloseCurrentPlan returns true when success has no subscription`() {
    assertTrue(shouldAutoCloseCurrentPlan(QueryState.Success(null)))
  }

  @Test
  fun `shouldAutoCloseCurrentPlan returns true when success is missing expiration`() {
    assertTrue(
      shouldAutoCloseCurrentPlan(
        QueryState.Success(
          SubscriptionSnapshot(
            id = "subscription-id",
            expiresAt = null,
          ),
        ),
      ),
    )
  }

  @Test
  fun `shouldAutoCloseCurrentPlan returns false when subscription can render`() {
    assertFalse(
      shouldAutoCloseCurrentPlan(
        QueryState.Success(
          SubscriptionSnapshot(
            id = "subscription-id",
            expiresAt = Instant.parse("2026-04-12T00:00:00Z"),
          ),
        ),
      ),
    )
  }

  @Test
  fun `enrollPlanSectionLabels includes current subscription label when no subscription exists`() {
    assertEquals(
      listOf("현재 이용 중인 이용권", "FULL ACCESS"),
      enrollPlanSectionLabels(hasSubscription = false),
    )
  }

  @Test
  fun `enrollPlanSectionLabels omits current subscription label when subscription exists`() {
    assertEquals(
      listOf("FULL ACCESS"),
      enrollPlanSectionLabels(hasSubscription = true),
    )
  }

  @Test
  fun `currentPlanDetailLines returns trial expiration copy`() {
    val lines = currentPlanDetailLines(
      availability = SubscriptionAvailability.Trial,
      fee = 0,
      state = SubscriptionState.Active,
      expiresAt = Instant.parse("2026-04-12T00:00:00Z"),
    )

    assertEquals(
      listOf("무료 체험이 2026년 04월 12일에 종료돼요."),
      lines,
    )
  }

  @Test
  fun `currentPlanDetailLines returns active paid renewal copy`() {
    val lines = currentPlanDetailLines(
      availability = SubscriptionAvailability.InAppPurchase,
      fee = 12900,
      state = SubscriptionState.Active,
      expiresAt = Instant.parse("2026-04-12T00:00:00Z"),
    )

    assertEquals(
      listOf(
        "이용권 가격: 12,900원",
        "다음 결제일: 2026년 04월 12일",
      ),
      lines,
    )
  }

  @Test
  fun `currentPlanDetailLines returns canceling paid copy`() {
    val lines = currentPlanDetailLines(
      availability = SubscriptionAvailability.InAppPurchase,
      fee = 12900,
      state = SubscriptionState.Canceled,
      expiresAt = Instant.parse("2026-04-12T00:00:00Z"),
    )

    assertEquals(
      listOf(
        "이용권 가격: 12,900원",
        "해지 예정일: 2026년 04월 12일",
      ),
      lines,
    )
  }

  @Test
  fun `currentPlanFooter returns purchase actions for in app purchase`() {
    val footer = currentPlanFooter(SubscriptionAvailability.InAppPurchase)

    assertIs<CurrentPlanFooter.Actions>(footer)
    assertEquals(listOf("해지하기", "변경하기"), footer.labels)
  }

  @Test
  fun `currentPlanFooter returns website note for billing key plans`() {
    val footer = currentPlanFooter(SubscriptionAvailability.BillingKey)

    assertEquals(
      CurrentPlanFooter.Note("웹사이트에서 가입한 이용권이에요.\n정보 변경이 필요할 경우 웹사이트에서 진행해주세요."),
      footer,
    )
  }

  @Test
  fun `currentPlanFooter returns support note for manual plans`() {
    val footer = currentPlanFooter(SubscriptionAvailability.Manual)

    assertEquals(
      CurrentPlanFooter.Note("정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요."),
      footer,
    )
  }

  @Test
  fun `currentPlanFooter returns upgrade action for trial`() {
    val footer = currentPlanFooter(SubscriptionAvailability.Trial)

    assertEquals(CurrentPlanFooter.Upgrade("지금 업그레이드"), footer)
  }

  @Test
  fun `cancelPlanBodyText includes formatted expiration date and plan name`() {
    val text = cancelPlanBodyText(
      planName = "타이피 FULL ACCESS",
      expiresAt = Instant.parse("2026-04-12T00:00:00Z"),
    )

    assertEquals(
      "지금 해지하더라도 2026년 04월 12일까지는 계속해서 타이피 FULL ACCESS 혜택을 이용할 수 있어요.",
      text,
    )
  }

  @Test
  fun `subscription state helpers keep loading unknown until state resolves`() {
    assertNull(QueryState.Loading.hasSubscriptionOrNull())
    assertNull(QueryState.Loading.subscriptionSummaryOrNull())
  }

  @Test
  fun `subscription state helpers keep error unknown instead of defaulting to basic`() {
    val error = QueryState.Error(Exception("failed"))

    assertNull(error.hasSubscriptionOrNull())
    assertNull(error.subscriptionSummaryOrNull())
  }

  @Test
  fun `subscription state helpers derive basic summary only after success without subscription`() {
    val state = QueryState.Success<SubscriptionSnapshot?>(null)

    assertEquals(false, state.hasSubscriptionOrNull())
    assertEquals(
      SubscriptionSummary(
        hasSubscription = false,
        subscriptionName = "타이피 BASIC ACCESS",
      ),
      state.subscriptionSummaryOrNull(),
    )
  }

  @Test
  fun `subscription state helpers derive full access summary from active subscription`() {
    val state = QueryState.Success(
      SubscriptionSnapshot(
        id = "subscription",
        planName = "타이피 FULL ACCESS",
      ),
    )

    assertEquals(true, state.hasSubscriptionOrNull())
    assertEquals(
      SubscriptionSummary(
        hasSubscription = true,
        subscriptionName = "타이피 FULL ACCESS",
      ),
      state.subscriptionSummaryOrNull(),
    )
  }
}

private fun relativeDate(
  date: LocalDate,
  days: Int,
): LocalDate {
  return LocalDate.fromEpochDays(date.toEpochDays() + days)
}
