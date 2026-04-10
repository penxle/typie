package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import co.typie.ext.clickable
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
internal fun BaseModal(
  title: String,
  message: String,
  onDismissRequest: suspend () -> Unit,
  actions: @Composable RowScope.() -> Unit,
) {
  val scope = rememberCoroutineScope()

  Dialog(onDismissRequest = { scope.launch { onDismissRequest() } }) {
    Column(
      modifier =
        Modifier.width(280.dp)
          .clip(RoundedCornerShape(16.dp))
          .background(AppTheme.colors.surfaceRaised),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      Column(
        modifier = Modifier.padding(start = 28.dp, end = 28.dp, top = 32.dp, bottom = 28.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        Text(title, style = AppTheme.typography.title)
        Spacer(Modifier.height(6.dp))
        Text(message, style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
      }

      Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderSubtle))

      Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        content = actions,
      )
    }
  }
}

@Composable
internal fun RowScope.BaseModalActionButton(
  text: String,
  color: Color = AppTheme.colors.textPrimary,
  onClick: suspend () -> Unit,
) {
  Box(
    modifier = Modifier.weight(1f).clickable(onClick).padding(vertical = 14.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(text = text, style = AppTheme.typography.action, color = color)
  }
}

@Composable
internal fun BaseModalActionDivider() {
  Box(modifier = Modifier.width(1.dp).height(48.dp).background(AppTheme.colors.borderSubtle))
}
