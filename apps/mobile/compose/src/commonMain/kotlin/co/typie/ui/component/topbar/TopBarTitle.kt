package co.typie.ui.component.topbar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
fun TopBarTitle(
  title: String,
  modifier: Modifier = Modifier,
  subtitle: String? = null,
  icon: IconData? = null,
) {
  val hasSubtitle = !subtitle.isNullOrEmpty()
  val shape = AppShapes.squircle(AppShapes.full)
  val bg = TopBarDefaults.controlBackgroundColor()
  val borderColor = TopBarDefaults.controlBorderColor()

  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier =
      modifier
        .fillMaxWidth()
        .height(TopBarDefaults.TitleHeight)
        .border(1.dp, borderColor, shape)
        .background(bg, shape)
        .padding(horizontal = TopBarDefaults.TitleHorizontalPadding),
  ) {
    if (icon != null) {
      Icon(
        icon = icon,
        modifier = Modifier.size(TopBarDefaults.TitleIconSize),
        tint = AppTheme.colors.textMuted,
      )
      Spacer(Modifier.width(TopBarDefaults.TitleIconGap))
    }

    Column(verticalArrangement = Arrangement.Center, modifier = Modifier.weight(1f)) {
      Text(
        text = title,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        style = AppTheme.typography.title.copy(fontSize = 15.sp),
      )
      if (hasSubtitle) {
        Spacer(Modifier.height(1.dp))
        Text(
          text = subtitle,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          style = AppTheme.typography.caption.copy(fontSize = 12.sp),
          color = AppTheme.colors.textMuted,
        )
      }
    }
  }
}
