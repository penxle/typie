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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.SubscriptionCelebrationSheet
import co.typie.domain.subscription.SubscriptionFeatureList
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.basicPlanFeatures
import co.typie.domain.subscription.fullPlanFeatures
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.thenIfNotNull
import co.typie.ext.verticalScroll
import co.typie.graphql.type.PlanAvailability
import co.typie.icons.Lucide
import co.typie.platform.PurchaseProduct
import co.typie.platform.activityContext
import co.typie.result.onErr
import co.typie.result.onOk
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
import co.typie.ui.component.loader.LocalLoader
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
fun EnrollPlanScreen() {
  val model = viewModel { EnrollPlanViewModel() }

  val scrollState = rememberScrollState()

  val dialog = LocalDialog.current
  val sheet = LocalSheet.current
  val toast = LocalToast.current
  val loader = LocalLoader.current

  val context = activityContext()

  LaunchedEffect(model) {
    model.events.collect { event ->
      when (event) {
        EnrollPlanEvent.PurchaseCompleted -> {
          sheet.present {
            SubscriptionCelebrationSheet(title = "구독이 시작됐어요!", message = "타이피의 모든 기능을 자유롭게 이용해보세요.")
          }
        }
      }
    }
  }

  ProvideTopBar(
    center = { Text("이용권 구매/변경", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      val subscription = SubscriptionService.subscription

      Text(
        text = "이용권 구매/변경",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      if (subscription == null) {
        SectionTitle("현재 이용 중인 이용권")

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            Row(
              modifier = Modifier.fillMaxWidth(),
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              Text(
                text = "타이피 BASIC ACCESS",
                style = AppTheme.typography.title,
                modifier = Modifier.weight(1f),
              )

              StatusBadge("현재 이용 중")
            }

            CardDivider(inset = 0.dp)

            SubscriptionFeatureList(features = basicPlanFeatures)
          }
        }
      }

      SectionTitle("FULL ACCESS")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.fillMaxWidth()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(20.dp),
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

              if (subscription?.availability == PlanAvailability.TRIAL) {
                StatusBadge("무료 체험 중")
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
            if (model.query.data.me.canStartTrial) {
              Button(
                text = "2주 무료 체험하기",
                leading = { color ->
                  Icon(icon = Lucide.Zap, modifier = Modifier.size(16.dp), tint = color)
                },
                onClick = {
                  val result =
                    dialog.confirm(
                      title = "무료 체험을 시작하시겠어요?",
                      message = "결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요.",
                      confirmText = "시작하기",
                    )

                  if (result is DialogResult.Resolved) {
                    loader.runWith {
                      model
                        .enrollTrial()
                        .withDefaultExceptionHandler(toast)
                        .onOk {
                          sheet.present {
                            SubscriptionCelebrationSheet(
                              title = "무료 체험이 시작됐어요!",
                              message = "2주간 타이피의 모든 기능을 자유롭게 이용해보세요.",
                            )
                          }
                        }
                        .onErr {
                          when (it) {
                            EnrollPlanError.SubscriptionHistoryExists ->
                              toast.error("이미 구독 기록이 존재해요.")

                            EnrollPlanError.TrialAlreadyUsed -> toast.error("이미 무료 체험을 사용한 적이 있어요.")
                          }
                        }
                    }
                  }
                },
              )
            }

            SubscriptionPurchaseRow(
              label = "1개월 구독하기",
              product = model.monthlyProduct,
              isActive = subscription?.planId == "PL0FL1MAP",
              onClick = { loader.runWith { context(context) { model.purchase(it) } } },
            )

            SubscriptionPurchaseRow(
              label = "1년 구독하기",
              product = model.yearlyProduct,
              isActive = subscription?.planId == "PL0FL1YAP",
              onClick = { loader.runWith { context(context) { model.purchase(it) } } },
            )
          }
        }
      }
    }
  }
}

@Composable
private fun StatusBadge(label: String) {
  Box(
    modifier =
      Modifier.background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.sm))
        .padding(horizontal = 8.dp, vertical = 4.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text = label, style = AppTheme.typography.micro, color = AppTheme.colors.textDefault)
  }
}

@Composable
private fun SubscriptionPurchaseRow(
  label: String,
  product: PurchaseProduct?,
  isActive: Boolean,
  onClick: suspend (PurchaseProduct) -> Unit,
) {
  InteractionScope {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.md))
          .thenIfNotNull(product) { clickable { onClick(it) } }
          .padding(12.dp)
          .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
      Text(text = label, style = AppTheme.typography.action)

      if (isActive) {
        StatusBadge("현재 이용 중")
      }

      Spacer(Modifier.weight(1f))

      if (product != null) {
        Text(text = product.price, style = AppTheme.typography.action)

        Icon(
          icon = Lucide.ChevronRight,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textMuted,
        )
      } else {
        Spinner()
      }
    }
  }
}

@Composable
private fun Spinner() {
  val transition = rememberInfiniteTransition()
  val colors = AppTheme.colors

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
        color = colors.textMuted,
        startAngle = rotation,
        sweepAngle = 270f,
        useCenter = false,
        style = Stroke(width = 1.5.dp.toPx(), cap = StrokeCap.Round),
      )
    }
  }
}
