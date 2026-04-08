package co.typie.screen.subscription

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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.platform.PurchasePlanInterval
import co.typie.platform.PurchaseProduct
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun EnrollPlanScreen() {
  val bottomSheetHost = LocalBottomSheetHost.current
  val currentSubscriptionStore = koinInject<CurrentSubscriptionStore>()
  val subscriptionService = koinInject<SubscriptionService>()
  val model = koinViewModel<EnrollPlanViewModel>()
  val scrollState = rememberScrollState()
  var showTrialStartConfirm by remember { mutableStateOf(false) }
  val currentSubscriptionState by currentSubscriptionStore.state.collectAsState()

  LaunchedEffect(model.celebration) {
    val celebration = model.celebration ?: return@LaunchedEffect
    bottomSheetHost.show {
      SubscriptionCelebrationSheet(
        title = celebration.title,
        message = celebration.message,
      )
    }
    model.consumeCelebration()
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("이용권 구매/변경", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (subscriptionService.hasQueryError(model.query.state)) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    scrollState = scrollState,
    loading = subscriptionService.isQueryLoading(model.query.state) || currentSubscriptionState !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    val currentSubscription = (currentSubscriptionState as? QueryState.Success)?.data
    val currentPlanId = currentSubscription?.planId
    val hasSubscription = currentSubscription != null
    val isOnTrial = currentSubscription?.availability == SubscriptionAvailability.Trial
    val canStartTrial = subscriptionService.canStartTrial(model.query.data.me.canStartTrial)

    Text(
      text = "이용권 구매/변경",
      style = AppTheme.typography.display,
      modifier = Modifier.padding(top = 4.dp),
    )

    if (!hasSubscription) {
      SectionTitle("현재 이용 중인 이용권")

      SubscriptionPlanCard(
        title = "타이피 BASIC ACCESS",
        badge = SubscriptionStatusBadge.Current,
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
        showTrialStartConfirm = true
      },
      onPurchaseMonthly = { product ->
        // TODO: Mixpanel enroll_plan_try / Appsflyer initiate_subscription
        model.purchase(product)
      },
      onPurchaseYearly = { product ->
        // TODO: Mixpanel enroll_plan_try / Appsflyer initiate_subscription
        model.purchase(product)
      },
    )

    Spacer(Modifier.height(72.dp))
  }

  if (showTrialStartConfirm) {
    ConfirmModal(
      title = TRIAL_START_CONFIRM_TITLE,
      message = TRIAL_START_CONFIRM_MESSAGE,
      confirmText = TRIAL_START_CONFIRM_ACTION,
      onConfirm = {
        showTrialStartConfirm = false
        model.startTrial()
      },
      onDismiss = { showTrialStartConfirm = false },
    )
  }
}

@Composable
private fun SubscriptionPlanCard(
  title: String,
  features: List<SubscriptionFeature>,
  badge: SubscriptionStatusBadge? = null,
) {
  CardSurface(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier
        .fillMaxWidth()
        .padding(18.dp),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Text(
          text = title,
          style = AppTheme.typography.title,
          modifier = Modifier.weight(1f),
        )

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
  CardSurface(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column(
        modifier = Modifier
          .fillMaxWidth()
          .padding(18.dp),
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
            SubscriptionStatusBadgeChip(SubscriptionStatusBadge.Trial)
          }
        }

        CardDivider(inset = 0.dp)

        SubscriptionFeatureList(features = fullPlanFeatures)
      }

      CardDivider()

      Column(
        modifier = Modifier
          .fillMaxWidth()
          .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        if (canStartTrial) {
          Button(
            text = "2주 무료 체험하기",
            leading = { color ->
              Icon(
                icon = Lucide.Zap,
                modifier = Modifier.size(16.dp),
                tint = color,
              )
            },
            onClick = onStartTrial,
          )
        }

        SubscriptionPurchaseRow(
          label = "1개월 구독하기",
          product = monthlyProduct,
          productsLoaded = productsLoaded,
          isActive = isCurrentFullPlan(
            currentPlanId = currentPlanId,
            interval = PurchasePlanInterval.Monthly,
          ),
          onClick = onPurchaseMonthly,
        )

        SubscriptionPurchaseRow(
          label = "1년 구독하기",
          product = yearlyProduct,
          productsLoaded = productsLoaded,
          isActive = isCurrentFullPlan(
            currentPlanId = currentPlanId,
            interval = PurchasePlanInterval.Yearly,
          ),
          onClick = onPurchaseYearly,
        )
      }
    }
  }
}

@Composable
private fun SubscriptionStatusBadgeChip(
  status: SubscriptionStatusBadge,
) {
  Box(
    modifier = Modifier
      .clip(RoundedCornerShape(6.dp))
      .background(AppTheme.colors.brandSubtle)
      .padding(horizontal = 8.dp, vertical = 4.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = subscriptionStatusBadgeLabel(status),
      style = AppTheme.typography.micro,
      color = AppTheme.colors.textOnBrandSubtle,
    )
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
  val productState = subscriptionProductState(
    product = product,
    productsLoaded = productsLoaded,
  )

  InteractionScope {
    Row(
      modifier = Modifier
        .fillMaxWidth()
        .clip(RoundedCornerShape(10.dp))
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
      Text(
        text = label,
        style = AppTheme.typography.action,
      )

      if (isActive) {
        SubscriptionStatusBadgeChip(SubscriptionStatusBadge.Current)
      }

      Spacer(Modifier.weight(1f))

      when (productState) {
        SubscriptionProductState.Loading -> SubscriptionPriceSpinner()
        SubscriptionProductState.Unavailable -> Text(
          text = PRODUCT_UNAVAILABLE_MESSAGE,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )
        SubscriptionProductState.Available -> {
          Text(
            text = product!!.price,
            style = AppTheme.typography.action,
          )

          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(16.dp),
            tint = AppTheme.colors.textSecondary,
          )
        }
      }
    }
  }
}

@Composable
private fun SubscriptionPriceSpinner() {
  val transition = rememberInfiniteTransition()
  val color = AppTheme.colors.textSecondary
  val rotation by transition.animateFloat(
    initialValue = 0f,
    targetValue = 360f,
    animationSpec = infiniteRepeatable(
      animation = tween(1000, easing = LinearEasing),
      repeatMode = RepeatMode.Restart,
    ),
  )

  Box(
    modifier = Modifier.size(16.dp),
    contentAlignment = Alignment.Center,
  ) {
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

private const val PRODUCT_UNAVAILABLE_MESSAGE = "불러오지 못했어요"
