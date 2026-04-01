package co.typie.screen.subscription

import co.typie.datetime.formatKoreanDate
import co.typie.graphql.type.SubscriptionState as GraphqlSubscriptionState
import co.typie.icons.Lucide
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.route.Route
import co.typie.ui.icon.IconData
import kotlin.time.Instant

const val FULL_ACCESS_MONTHLY_PLAN_ID = "PL0FL1MAP"
const val FULL_ACCESS_YEARLY_PLAN_ID = "PL0FL1YAP"
const val FULL_ACCESS_MONTHLY_STORE_PRODUCT_ID = "pl0fl1map"
const val FULL_ACCESS_YEARLY_STORE_PRODUCT_ID = "pl0fl1yap"
const val FULL_ACCESS_GOOGLE_PLAY_PRODUCT_ID = "plan.full"
const val TRIAL_START_CONFIRM_TITLE = "무료 체험을 시작하시겠어요?"
const val TRIAL_START_CONFIRM_MESSAGE = "결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요."
const val TRIAL_START_CONFIRM_ACTION = "시작하기"

data class SubscriptionFeature(
  val icon: IconData,
  val label: String,
)

data class SubscriptionSnapshot(
  val id: String,
  val state: SubscriptionState? = null,
  val startsAt: Instant? = null,
  val expiresAt: Instant? = null,
  val planId: String? = null,
  val planName: String? = null,
  val fee: Int? = null,
  val availability: SubscriptionAvailability? = null,
)

enum class SubscriptionStatusBadge {
  Current,
  Trial,
}

enum class SubscriptionProductState {
  Loading,
  Available,
  Unavailable,
}

enum class SubscriptionAvailability {
  InAppPurchase,
  BillingKey,
  Manual,
  Trial,
}

enum class SubscriptionState {
  Active,
  Canceled,
}

enum class SubscriptionEntryDestination {
  CurrentPlan,
  EnrollPlan,
}

sealed interface CurrentPlanFooter {
  data class Actions(
    val labels: List<String>,
  ) : CurrentPlanFooter

  data class Note(
    val text: String,
  ) : CurrentPlanFooter

  data class Upgrade(
    val label: String,
  ) : CurrentPlanFooter
}

val basicPlanFeatures = listOf(
  SubscriptionFeature(icon = Lucide.BookOpenText, label = "200,000자까지 작성 가능"),
  SubscriptionFeature(icon = Lucide.Images, label = "100MB까지 파일 업로드 가능"),
)

val fullPlanFeatures = listOf(
  SubscriptionFeature(icon = Lucide.BookOpenText, label = "무제한 글자 수"),
  SubscriptionFeature(icon = Lucide.Images, label = "무제한 파일 업로드"),
  SubscriptionFeature(icon = Lucide.SpellCheck, label = "맞춤법 검사"),
  SubscriptionFeature(icon = Lucide.Link, label = "커스텀 게시 주소"),
  SubscriptionFeature(icon = Lucide.Type, label = "커스텀 폰트 업로드"),
  SubscriptionFeature(icon = Lucide.FlaskConical, label = "베타 기능 우선 접근"),
  SubscriptionFeature(icon = Lucide.Headset, label = "문제 발생 시 우선 지원"),
  SubscriptionFeature(icon = Lucide.Sprout, label = "디스코드 커뮤니티 참여"),
  SubscriptionFeature(icon = Lucide.Ellipsis, label = "그리고 더 많은 혜택"),
)

fun subscriptionPlanId(interval: PurchasePlanInterval): String {
  return when (interval) {
    PurchasePlanInterval.Monthly -> FULL_ACCESS_MONTHLY_PLAN_ID
    PurchasePlanInterval.Yearly -> FULL_ACCESS_YEARLY_PLAN_ID
  }
}

fun isCurrentFullPlan(
  currentPlanId: String?,
  interval: PurchasePlanInterval,
): Boolean {
  return currentPlanId == subscriptionPlanId(interval)
}

fun shouldShowPurchaseCelebration(
  originalSubscriptionId: String?,
  originalPlanId: String?,
  updatedSubscriptionId: String,
  updatedPlanId: String,
): Boolean {
  return originalSubscriptionId != updatedSubscriptionId || originalPlanId != updatedPlanId
}

fun subscriptionEntryDestination(hasSubscription: Boolean): SubscriptionEntryDestination {
  return if (hasSubscription) SubscriptionEntryDestination.CurrentPlan else SubscriptionEntryDestination.EnrollPlan
}

fun enrollPlanSectionLabels(hasSubscription: Boolean): List<String> {
  return buildList {
    if (!hasSubscription) {
      add("현재 이용 중인 이용권")
    }
    add("FULL ACCESS")
  }
}

fun subscriptionStatusBadgeLabel(status: SubscriptionStatusBadge): String {
  return when (status) {
    SubscriptionStatusBadge.Current -> "현재 이용 중"
    SubscriptionStatusBadge.Trial -> "무료 체험 중"
  }
}

fun subscriptionProductState(
  product: PurchaseProduct?,
  productsLoaded: Boolean,
): SubscriptionProductState {
  return when {
    product != null -> SubscriptionProductState.Available
    productsLoaded -> SubscriptionProductState.Unavailable
    else -> SubscriptionProductState.Loading
  }
}

fun subscriptionRoute(destination: SubscriptionEntryDestination): Route {
  return when (destination) {
    SubscriptionEntryDestination.CurrentPlan -> Route.CurrentPlan
    SubscriptionEntryDestination.EnrollPlan -> Route.EnrollPlan
  }
}

fun currentPlanDetailLines(
  availability: SubscriptionAvailability,
  fee: Int,
  state: SubscriptionState,
  expiresAt: Instant,
): List<String> {
  if (availability == SubscriptionAvailability.Trial) {
    return listOf("무료 체험이 ${expiresAt.formatKoreanDate()}에 종료돼요.")
  }

  return listOf(
    "이용권 가격: ${fee.formatGrouped()}원",
    if (state == SubscriptionState.Active) {
      "다음 결제일: ${expiresAt.formatKoreanDate()}"
    } else {
      "해지 예정일: ${expiresAt.formatKoreanDate()}"
    },
  )
}

fun currentPlanFooter(
  availability: SubscriptionAvailability,
): CurrentPlanFooter {
  return when (availability) {
    SubscriptionAvailability.InAppPurchase -> CurrentPlanFooter.Actions(listOf("해지하기", "변경하기"))
    SubscriptionAvailability.BillingKey -> CurrentPlanFooter.Note(
      "웹사이트에서 가입한 이용권이에요.\n정보 변경이 필요할 경우 웹사이트에서 진행해주세요.",
    )
    SubscriptionAvailability.Manual -> CurrentPlanFooter.Note(
      "정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.",
    )
    SubscriptionAvailability.Trial -> CurrentPlanFooter.Upgrade("지금 업그레이드")
  }
}

fun cancelPlanBodyText(
  planName: String,
  expiresAt: Instant,
): String {
  return "지금 해지하더라도 ${expiresAt.formatKoreanDate()}까지는 계속해서 $planName 혜택을 이용할 수 있어요."
}

fun shouldCloseCancelPlanAfterStoreReturn(
  awaitingStoreResult: Boolean,
  subscriptionState: GraphqlSubscriptionState?,
): Boolean {
  if (!awaitingStoreResult) {
    return false
  }

  return subscriptionState == null || subscriptionState != GraphqlSubscriptionState.ACTIVE
}

private fun Int.formatGrouped(): String {
  val text = toString()
  val builder = StringBuilder()

  text.forEachIndexed { index, char ->
    if (index > 0 && (text.length - index) % 3 == 0) {
      builder.append(',')
    }
    builder.append(char)
  }

  return builder.toString()
}
