package co.typie.screen.subscription

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
fun BottomSheetScope<Unit>.SubscriptionCelebrationSheet(
  title: String,
  message: String,
) {
  Column(
    modifier = Modifier.padding(horizontal = 20.dp),
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    CelebrationBadgeRow()

    Column(
      modifier = Modifier.fillMaxWidth(),
      horizontalAlignment = Alignment.CenterHorizontally,
      verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
      Text(
        text = title,
        style = AppTheme.typography.title,
      )

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

@Composable
fun SubscriptionFeatureList(
  features: List<SubscriptionFeature>,
  modifier: Modifier = Modifier,
  iconSize: Dp = 18.dp,
  rowSpacing: Dp = 10.dp,
) {
  Column(
    modifier = modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(rowSpacing),
  ) {
    features.forEach { feature ->
      SubscriptionFeatureItem(
        feature = feature,
        iconSize = iconSize,
      )
    }
  }
}

@Composable
private fun CelebrationBadgeRow() {
  val icons = listOf(
    Lucide.Crown,
    Lucide.Tag,
    Lucide.Star,
    Lucide.Key,
    Lucide.Gift,
  )

  Box(
    modifier = Modifier.fillMaxWidth(),
    contentAlignment = Alignment.Center,
  ) {
    Row(
      horizontalArrangement = Arrangement.spacedBy((-10).dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      icons.forEach { icon ->
        CelebrationBadge(icon = icon)
      }
    }
  }
}

@Composable
private fun CelebrationBadge(
  icon: IconData,
) {
  val borderColor = AppTheme.colors.surfaceDefault
  val backgroundColor = AppTheme.colors.textPrimary
  val iconTint = AppTheme.colors.surfaceDefault

  Box(
    modifier = Modifier
      .size(32.dp)
      .clip(CircleShape)
      .border(2.dp, borderColor, CircleShape),
    contentAlignment = Alignment.Center,
  ) {
    Canvas(
      modifier = Modifier.matchParentSize(),
    ) {
      drawCircle(color = backgroundColor)
    }

    Icon(
      icon = icon,
      modifier = Modifier.size(16.dp),
      tint = iconTint,
    )
  }
}

@Composable
private fun SubscriptionFeatureItem(
  feature: SubscriptionFeature,
  iconSize: Dp,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    Icon(
      icon = feature.icon,
      modifier = Modifier.size(iconSize),
      tint = AppTheme.colors.textSecondary,
    )

    Text(
      text = feature.label,
      style = AppTheme.typography.body,
    )
  }
}
