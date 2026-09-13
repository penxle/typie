package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
internal fun EditorExternalElementPlaceholder(
  icon: IconData,
  text: String,
  trailing: (@Composable () -> Unit)? = null,
) {
  val shape = AppShapes.rounded(4.dp)
  Row(
    modifier =
      Modifier.height(48.dp)
        .fillMaxWidth()
        .clip(shape)
        .background(AppTheme.colors.surfaceInset, shape)
        .padding(horizontal = 14.dp, vertical = 12.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = icon,
      contentDescription = null,
      modifier = Modifier.size(20.dp),
      tint = AppTheme.colors.textHint,
    )
    Text(
      text = text,
      modifier = Modifier.padding(start = 12.dp).weight(1f),
      color = AppTheme.colors.textHint,
      style = AppTheme.typography.body.copy(fontSize = 14.sp),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    trailing?.invoke()
  }
}
