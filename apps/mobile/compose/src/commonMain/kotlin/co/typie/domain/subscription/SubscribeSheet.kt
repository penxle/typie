package co.typie.domain.subscription

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Button
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.PaperlogyFontFamily

sealed interface SubscribeSheetResult {
  data object Subscribe : SubscribeSheetResult
}

@Composable
context(_: SheetScope<SubscribeSheetResult>)
fun SubscribeSheet() {
  SheetLayout(
    padding = SheetPadding(header = PaddingValues(0.dp)),
    handle = false,
    headerBackgroundColor = AppTheme.colors.surfaceInverse,
    header = { SubscribeHero() },
    footer = {
      Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Button(
          text = "구독하기",
          textStyle = TextStyle(fontFamily = PaperlogyFontFamily),
          height = 54.dp,
          onClick = { complete(SubscribeSheetResult.Subscribe) },
        )
        SheetBarTextButton(
          text = "나중에 할게요",
          modifier = Modifier.fillMaxWidth(),
          color = AppTheme.colors.textMuted,
          onClick = { dismiss() },
        )
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
      PlanUpgradeBenefitList(benefits = PlanUpgradeBenefit.entries)
    }
  }
}

@Composable
private fun SubscribeHero() {
  Column(
    modifier =
      Modifier.fillMaxWidth()
        .background(AppTheme.colors.surfaceInverse)
        .padding(horizontal = 20.dp, vertical = 24.dp),
    verticalArrangement = Arrangement.spacedBy(6.dp),
  ) {
    Text(
      text = "타이피 구독하기",
      style = AppTheme.typography.heading.copy(fontFamily = PaperlogyFontFamily),
      color = AppTheme.colors.textOnInverse,
    )
    Text(
      text = "모든 기능을 제한 없이 이용해 보세요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textOnInverse.copy(alpha = .7f),
    )
  }
}

@Composable
private fun PlanUpgradeBenefitList(benefits: List<PlanUpgradeBenefit>) {
  Column(modifier = Modifier.fillMaxWidth()) {
    benefits.forEachIndexed { index, benefit ->
      Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.Top,
      ) {
        Box(
          modifier =
            Modifier.size(28.dp).background(AppTheme.colors.surfaceInset, RoundedCornerShape(8.dp)),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = benefit.icon,
            modifier = Modifier.size(15.dp),
            tint = AppTheme.colors.textDefault,
          )
        }
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
          Text(
            text = benefit.title,
            style = AppTheme.typography.label.copy(fontFamily = PaperlogyFontFamily),
            color = AppTheme.colors.textDefault,
          )
          Text(
            text = benefit.description,
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textMuted,
          )
        }
      }
      if (index < benefits.lastIndex) {
        Divider(color = AppTheme.colors.borderHairline)
      }
    }
  }
}
