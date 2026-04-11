package co.typie.screen.subscription

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.SheetInsetPolicy
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.theme.AppTheme

fun subscriptionCelebrationSheet(title: String, message: String): SheetPresentation<Unit> =
  sheetPresentation {
    SheetLayout(
      bodyInsetPolicy = SheetInsetPolicy.Container,
      header = { ActionHeader(title = title) },
      footer = { Button(text = "시작하기", onClick = { dismiss() }) },
    ) {
      SubscriptionBadgeRow()

      Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
      ) {
        Text(
          text = message,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )
      }

      Column(
        modifier =
          Modifier.fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        Text(text = "타이피 FULL ACCESS", style = AppTheme.typography.title)

        CardDivider(inset = 0.dp, color = AppTheme.colors.borderStrong)

        SubscriptionFeatureList(features = fullPlanFeatures, iconSize = 16.dp, rowSpacing = 8.dp)
      }
    }
  }
