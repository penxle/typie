package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppColors
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

enum class AlertBannerVariant {
  Default,
  Danger,
  Success,
}

@Immutable
private data class AlertBannerColors(val background: Color, val border: Color, val text: Color)

@Composable
fun AlertBanner(
  text: String,
  modifier: Modifier = Modifier,
  variant: AlertBannerVariant = AlertBannerVariant.Default,
  icon: IconData? = defaultAlertBannerIcon(variant),
) {
  val colors = AppTheme.colors.alertBannerColors(variant)
  val shape = AppShapes.rounded(AppShapes.md)

  Row(
    modifier =
      modifier
        .fillMaxWidth()
        .background(colors.background, shape)
        .border(1.dp, colors.border, shape)
        .padding(horizontal = 12.dp, vertical = 10.dp),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    if (icon != null) {
      Icon(icon = icon, modifier = Modifier.size(14.dp), tint = colors.text)
    }

    Text(
      text = text,
      style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W700),
      color = colors.text,
    )
  }
}

private fun AppColors.alertBannerColors(variant: AlertBannerVariant): AlertBannerColors =
  when (variant) {
    AlertBannerVariant.Default ->
      AlertBannerColors(
        background = surfaceInset,
        border = textDefault.copy(alpha = 0.14f),
        text = textDefault,
      )

    AlertBannerVariant.Danger ->
      AlertBannerColors(
        background = dangerSubtle,
        border = danger.copy(alpha = 0.14f),
        text = textOnDangerSubtle,
      )

    AlertBannerVariant.Success ->
      AlertBannerColors(
        background = successSubtle,
        border = success.copy(alpha = 0.14f),
        text = textOnSuccessSubtle,
      )
  }

private fun defaultAlertBannerIcon(variant: AlertBannerVariant): IconData? =
  when (variant) {
    AlertBannerVariant.Default,
    AlertBannerVariant.Danger -> Lucide.CircleAlert

    AlertBannerVariant.Success -> Lucide.Check
  }
