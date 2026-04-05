package co.typie.screen.subscription

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.theme.AppTheme

@Composable
fun BottomSheetScope<Unit>.SubscriptionCelebrationSheet(
  title: String,
  message: String,
) {
  BottomSheetScaffold(title = title) {
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
      modifier = Modifier
        .fillMaxWidth()
        .clip(RoundedCornerShape(8.dp))
        .border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
        .padding(16.dp),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Text(
        text = "타이피 FULL ACCESS",
        style = AppTheme.typography.title,
      )

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderStrong)

      SubscriptionFeatureList(
        features = fullPlanFeatures,
        iconSize = 16.dp,
        rowSpacing = 8.dp,
      )
    }

    Button(
      text = "시작하기",
      onClick = { dismiss() },
    )
  }
}
