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
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorExternalElementPlaceholder(
  icon: IconData,
  text: String,
  trailing: (@Composable () -> Unit)? = null,
) {
  Row(
    modifier =
      Modifier.height(scope.scaledDp(48f))
        .fillMaxWidth()
        .clip(scope.shape)
        .background(AppTheme.colors.surfaceInset, scope.shape)
        .padding(horizontal = scope.scaledDp(14f), vertical = scope.scaledDp(12f)),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = icon,
      contentDescription = null,
      modifier = Modifier.size(scope.scaledDp(20f)),
      tint = AppTheme.colors.textHint,
    )
    Text(
      text = text,
      modifier = Modifier.padding(start = scope.scaledDp(12f)).weight(1f),
      color = AppTheme.colors.textHint,
      style = AppTheme.typography.body.copy(fontSize = scope.scaledSp(14f)),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    trailing?.invoke()
  }
}
