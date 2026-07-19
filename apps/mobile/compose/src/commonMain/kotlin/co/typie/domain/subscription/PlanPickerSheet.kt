package co.typie.domain.subscription

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.platform.PurchaseProduct
import co.typie.platform.activityContext
import co.typie.ui.component.Button
import co.typie.ui.component.Text
import co.typie.ui.component.loader.LocalLoader
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.PaperlogyFontFamily

sealed interface PlanPickerSheetResult {
  data object Purchased : PlanPickerSheetResult
}

@Composable
context(_: SheetScope<PlanPickerSheetResult>)
fun PlanPickerSheet() {
  val loader = LocalLoader.current
  val context = activityContext()

  var selectedPlanId by remember { mutableStateOf("pl0fl1map") }
  val selectedProduct =
    if (selectedPlanId == "pl0fl1map") SubscriptionPurchaseService.monthlyProduct
    else SubscriptionPurchaseService.yearlyProduct

  LaunchedEffect(Unit) { SubscriptionPurchaseService.ensureProductsLoaded() }

  LaunchedEffect(Unit) {
    SubscriptionPurchaseService.completions.collect { complete(PlanPickerSheetResult.Purchased) }
  }

  SheetLayout(
    header = {
      Column(modifier = Modifier.fillMaxWidth().padding(top = 8.dp, bottom = 4.dp)) {
        Text(
          text = "플랜 선택",
          style = AppTheme.typography.title.copy(fontFamily = PaperlogyFontFamily),
          color = AppTheme.colors.textDefault,
        )
      }
    },
    footer = {
      Button(
        text = "결제하기",
        textStyle = TextStyle(fontFamily = PaperlogyFontFamily),
        height = 54.dp,
        enabled = selectedProduct != null,
        onClick = {
          selectedProduct?.let { product ->
            loader.runWith {
              val generation = SubscriptionPurchaseService.registrationGeneration
              if (context(context) { SubscriptionPurchaseService.purchase(product) }) {
                SubscriptionPurchaseService.awaitRegistration(generation)
              }
            }
          }
        },
      )
    },
  ) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
      verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
      if (SubscriptionPurchaseService.productsUnavailable) {
        Text(
          text = "지금은 스토어 결제를 이용할 수 없어요",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textMuted,
          textAlign = TextAlign.Center,
          modifier = Modifier.fillMaxWidth().padding(vertical = 24.dp),
        )
      } else {
        PlanOptionCard(
          label = "월간",
          billing = "매월 결제",
          badge = null,
          product = SubscriptionPurchaseService.monthlyProduct,
          priceSuffix = "/월",
          selected = selectedPlanId == "pl0fl1map",
          onClick = { selectedPlanId = "pl0fl1map" },
        )

        PlanOptionCard(
          label = "연간",
          billing = "매년 결제",
          badge = "2개월 무료",
          product = SubscriptionPurchaseService.yearlyProduct,
          priceSuffix = "/년",
          selected = selectedPlanId == "pl0fl1yap",
          onClick = { selectedPlanId = "pl0fl1yap" },
        )
      }
    }
  }
}

@Composable
private fun PlanOptionCard(
  label: String,
  billing: String,
  badge: String?,
  product: PurchaseProduct?,
  priceSuffix: String,
  selected: Boolean,
  onClick: () -> Unit,
) {
  val colors = AppTheme.colors

  val cardColor by animateColorAsState(if (selected) colors.surfaceInset else colors.surfaceDefault)
  val outlineColor by
    animateColorAsState(if (selected) colors.surfaceInverse else colors.borderDefault)
  val outlineWidth by animateDpAsState(if (selected) 1.5.dp else 1.dp)

  InteractionScope {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .background(cardColor, AppShapes.rounded(AppShapes.md))
          .border(outlineWidth, outlineColor, AppShapes.rounded(AppShapes.md))
          .clickable { onClick() }
          .padding(16.dp)
          .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Box(
        modifier =
          Modifier.size(22.dp)
            .background(
              if (selected) colors.surfaceInverse else Color.Transparent,
              AppShapes.circle,
            )
            .border(
              width = 1.5.dp,
              color = if (selected) Color.Transparent else colors.borderDefault,
              shape = AppShapes.circle,
            ),
        contentAlignment = Alignment.Center,
      ) {
        if (selected) {
          Icon(icon = Lucide.Check, modifier = Modifier.size(13.dp), tint = colors.textOnInverse)
        }
      }

      Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Row(
          horizontalArrangement = Arrangement.spacedBy(6.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Text(
            text = label,
            style = AppTheme.typography.action.copy(fontWeight = FontWeight.SemiBold),
            color = colors.textDefault,
          )

          if (badge != null) {
            Box(
              modifier =
                Modifier.background(colors.surfaceInverse, AppShapes.circle)
                  .padding(horizontal = 7.dp, vertical = 2.dp)
            ) {
              Text(
                text = badge,
                style = AppTheme.typography.micro.copy(fontWeight = FontWeight.SemiBold),
                color = colors.textOnInverse,
              )
            }
          }
        }

        Text(text = billing, style = AppTheme.typography.caption, color = colors.textMuted)
      }

      if (product != null) {
        Row(
          horizontalArrangement = Arrangement.spacedBy(2.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Text(
            text = product.price,
            style = AppTheme.typography.title.copy(fontFamily = PaperlogyFontFamily),
            color = colors.textDefault,
          )

          Text(text = priceSuffix, style = AppTheme.typography.caption, color = colors.textMuted)
        }
      } else {
        PriceSpinner(color = colors.textMuted)
      }
    }
  }
}

@Composable
private fun PriceSpinner(color: Color) {
  val transition = rememberInfiniteTransition()

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
