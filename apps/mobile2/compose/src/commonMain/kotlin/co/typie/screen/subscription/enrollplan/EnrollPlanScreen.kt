package co.typie.screen.subscription.enrollplan

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.FULL_ACCESS_MONTHLY_PLAN_ID
import co.typie.domain.subscription.FULL_ACCESS_YEARLY_PLAN_ID
import co.typie.domain.subscription.SubscriptionCelebrationSheet
import co.typie.domain.subscription.SubscriptionFeature
import co.typie.domain.subscription.SubscriptionFeatureList
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.domain.subscription.basicPlanFeatures
import co.typie.domain.subscription.fullPlanFeatures
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.type.PlanAvailability
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.result.DEFAULT_ERROR_MESSAGE
import co.typie.result.onErr
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.loader.LocalLoader
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun EnrollPlanScreen() {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val sheet = LocalSheet.current
  val toast = LocalToast.current
  val loader = LocalLoader.current
  val model = viewModel { EnrollPlanViewModel() }
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val currentSubscriptionState = SubscriptionService.state

  LaunchedEffect(model.showTrialCelebration) {
    if (!model.showTrialCelebration) return@LaunchedEffect
    sheet.present {
      SubscriptionCelebrationSheet(
        title = "무료 체험이 시작됐어요!",
        message = "2주간 타이피의 모든 기능을 자유롭게 이용해보세요.",
      )
    }
    model.consumeTrialCelebration()
  }

  LaunchedEffect(model.showPurchaseCelebration) {
    if (!model.showPurchaseCelebration) return@LaunchedEffect
    sheet.present {
      SubscriptionCelebrationSheet(title = "구독이 시작됐어요!", message = "타이피의 모든 기능을 자유롭게 이용해보세요.")
    }
    model.consumePurchaseCelebration()
  }

  LaunchedEffect(model.purchaseError) {
    model.purchaseError ?: return@LaunchedEffect
    toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
    model.consumePurchaseError()
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("이용권 구매/변경", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(
    loading =
      model.query.state !is QueryState.Success ||
        currentSubscriptionState is SubscriptionServiceState.Unknown
  ) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      val currentSubscription =
        (currentSubscriptionState as? SubscriptionServiceState.Subscribed)?.subscription
      val currentPlanId = currentSubscription?.planId
      val hasSubscription = currentSubscription != null
      val isOnTrial = currentSubscription?.availability == PlanAvailability.TRIAL
      val canStartTrial = model.query.data.me.canStartTrial

      Text(
        text = "이용권 구매/변경",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      if (!hasSubscription) {
        SectionTitle("현재 이용 중인 이용권")

        SubscriptionPlanCard(
          title = "타이피 BASIC ACCESS",
          badge = "현재 이용 중",
          features = basicPlanFeatures,
        )
      }

      SectionTitle("FULL ACCESS")

      FullAccessCard(
        isOnTrial = isOnTrial,
        canStartTrial = canStartTrial,
        currentPlanId = currentPlanId,
        productsLoaded = model.productsLoaded,
        monthlyProduct = model.products[PurchasePlanInterval.Monthly],
        yearlyProduct = model.products[PurchasePlanInterval.Yearly],
        onStartTrial = {
          val result =
            dialog.confirm(
              title = TRIAL_START_CONFIRM_TITLE,
              message = TRIAL_START_CONFIRM_MESSAGE,
              confirmText = TRIAL_START_CONFIRM_ACTION,
            )
          if (result is DialogResult.Resolved) {
            loader.runWith {
              model.startTrial().withDefaultExceptionHandler(toast).onErr { error ->
                when (error) {
                  EnrollPlanError.ServerError -> toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
                }
              }
            }
          }
        },
        onPurchaseMonthly = { product ->
          // TODO: Mixpanel enroll_plan_try / Appsflyer initiate_subscription
          scope.launch {
            loader.runWith { model.purchase(product).withDefaultExceptionHandler(toast) }
          }
        },
        onPurchaseYearly = { product ->
          // TODO: Mixpanel enroll_plan_try / Appsflyer initiate_subscription
          scope.launch {
            loader.runWith { model.purchase(product).withDefaultExceptionHandler(toast) }
          }
        },
      )

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun SubscriptionPlanCard(
  title: String,
  features: List<SubscriptionFeature>,
  badge: String? = null,
) {
  CardSurface(modifier = Modifier.fillMaxWidth()) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(18.dp),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Text(text = title, style = AppTheme.typography.title, modifier = Modifier.weight(1f))

        if (badge != null) {
          SubscriptionStatusBadgeChip(badge)
        }
      }

      CardDivider(inset = 0.dp)

      SubscriptionFeatureList(features = features)
    }
  }
}

@Composable
private fun FullAccessCard(
  isOnTrial: Boolean,
  canStartTrial: Boolean,
  currentPlanId: String?,
  productsLoaded: Boolean,
  monthlyProduct: PurchaseProduct?,
  yearlyProduct: PurchaseProduct?,
  onStartTrial: suspend () -> Unit,
  onPurchaseMonthly: suspend (PurchaseProduct) -> Unit,
  onPurchaseYearly: suspend (PurchaseProduct) -> Unit,
) {
  CardSurface(modifier = Modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth()) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        Row(
          modifier = Modifier.fillMaxWidth(),
          verticalAlignment = Alignment.CenterVertically,
          horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          Text(
            text = "타이피 FULL ACCESS",
            style = AppTheme.typography.title,
            modifier = Modifier.weight(1f),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )

          if (isOnTrial) {
            SubscriptionStatusBadgeChip("무료 체험 중")
          }
        }

        CardDivider(inset = 0.dp)

        SubscriptionFeatureList(features = fullPlanFeatures)
      }

      CardDivider()

      Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        if (canStartTrial) {
          Button(
            text = "2주 무료 체험하기",
            leading = { color ->
              Icon(icon = Lucide.Zap, modifier = Modifier.size(16.dp), tint = color)
            },
            onClick = onStartTrial,
          )
        }

        SubscriptionPurchaseRow(
          label = "1개월 구독하기",
          product = monthlyProduct,
          productsLoaded = productsLoaded,
          isActive = currentPlanId == FULL_ACCESS_MONTHLY_PLAN_ID,
          onClick = onPurchaseMonthly,
        )

        SubscriptionPurchaseRow(
          label = "1년 구독하기",
          product = yearlyProduct,
          productsLoaded = productsLoaded,
          isActive = currentPlanId == FULL_ACCESS_YEARLY_PLAN_ID,
          onClick = onPurchaseYearly,
        )
      }
    }
  }
}

@Composable
private fun SubscriptionStatusBadgeChip(label: String) {
  Box(
    modifier =
      Modifier.clip(AppShapes.rounded(AppShapes.sm))
        .background(AppTheme.colors.brandSubtle)
        .padding(horizontal = 8.dp, vertical = 4.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text = label, style = AppTheme.typography.micro, color = AppTheme.colors.textOnBrandSubtle)
  }
}

@Composable
private fun SubscriptionPurchaseRow(
  label: String,
  product: PurchaseProduct?,
  productsLoaded: Boolean,
  isActive: Boolean,
  onClick: suspend (PurchaseProduct) -> Unit,
) {
  InteractionScope {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .clip(AppShapes.rounded(AppShapes.md))
          .background(AppTheme.colors.surfaceSunken)
          .clickable {
            val currentProduct = product ?: return@clickable
            onClick(currentProduct)
          }
          .padding(13.dp)
          .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
      Text(text = label, style = AppTheme.typography.action)

      if (isActive) {
        SubscriptionStatusBadgeChip("현재 이용 중")
      }

      Spacer(Modifier.weight(1f))

      when {
        product != null -> {
          Text(text = product.price, style = AppTheme.typography.action)

          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textSecondary,
          )
        }
        productsLoaded ->
          Text(
            text = PRODUCT_UNAVAILABLE_MESSAGE,
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
          )
        else -> SubscriptionPriceSpinner()
      }
    }
  }
}

@Composable
private fun SubscriptionPriceSpinner() {
  val transition = rememberInfiniteTransition()
  val color = AppTheme.colors.textSecondary
  val rotation by
    transition.animateFloat(
      initialValue = 0f,
      targetValue = 360f,
      animationSpec =
        infiniteRepeatable(
          animation = tween(1000, easing = LinearEasing),
          repeatMode = RepeatMode.Restart,
        ),
    )

  Box(modifier = Modifier.size(16.dp), contentAlignment = Alignment.Center) {
    Canvas(Modifier.size(14.dp)) {
      drawArc(
        color = color,
        startAngle = rotation,
        sweepAngle = 270f,
        useCenter = false,
        style = Stroke(width = 1.5.dp.toPx(), cap = StrokeCap.Round),
      )
    }
  }
}

private const val TRIAL_START_CONFIRM_TITLE = "무료 체험을 시작하시겠어요?"
private const val TRIAL_START_CONFIRM_MESSAGE =
  "결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요."
private const val TRIAL_START_CONFIRM_ACTION = "시작하기"
private const val PRODUCT_UNAVAILABLE_MESSAGE = "불러오지 못했어요"
