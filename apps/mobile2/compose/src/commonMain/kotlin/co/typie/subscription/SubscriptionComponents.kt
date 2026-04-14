package co.typie.subscription

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

data class SubscriptionFeature(val icon: IconData, val label: String)

val basicPlanFeatures =
  listOf(
    SubscriptionFeature(icon = Lucide.BookOpenText, label = "200,000자까지 작성 가능"),
    SubscriptionFeature(icon = Lucide.Images, label = "100MB까지 파일 업로드 가능"),
  )

val fullPlanFeatures =
  listOf(
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
    features.forEach { feature -> SubscriptionFeatureItem(feature = feature, iconSize = iconSize) }
  }
}

@Composable
fun SubscriptionBadgeRow() {
  val icons = listOf(Lucide.Crown, Lucide.Tag, Lucide.Star, Lucide.Key, Lucide.Gift)

  Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
    Row(
      horizontalArrangement = Arrangement.spacedBy((-10).dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      icons.forEach { icon -> CelebrationBadge(icon = icon) }
    }
  }
}

@Composable
private fun CelebrationBadge(icon: IconData) {
  val borderColor = AppTheme.colors.surfaceDefault
  val backgroundColor = AppTheme.colors.textPrimary
  val iconTint = AppTheme.colors.surfaceDefault

  Box(
    modifier = Modifier.size(32.dp).clip(CircleShape).border(2.dp, borderColor, CircleShape),
    contentAlignment = Alignment.Center,
  ) {
    Canvas(modifier = Modifier.matchParentSize()) { drawCircle(color = backgroundColor) }

    Icon(icon = icon, modifier = Modifier.size(16.dp), tint = iconTint)
  }
}

@Composable
private fun SubscriptionFeatureItem(feature: SubscriptionFeature, iconSize: Dp) {
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

    Text(text = feature.label, style = AppTheme.typography.body)
  }
}
