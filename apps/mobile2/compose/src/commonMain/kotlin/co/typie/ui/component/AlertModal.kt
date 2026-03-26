package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import co.typie.ext.clickable
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun AlertModal(
  title: String,
  message: String,
  confirmText: String = "확인",
  onConfirm: suspend () -> Unit,
  onDismiss: (suspend () -> Unit)? = null,
) {
  val scope = rememberCoroutineScope()
  val dismissAction = onDismiss ?: onConfirm

  Dialog(
    onDismissRequest = {
      scope.launch {
        dismissAction()
      }
    },
  ) {
    Column(
      modifier = Modifier
        .width(280.dp)
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
        Text(
          message,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )
      }

      Box(
        Modifier
          .fillMaxWidth()
          .height(1.dp)
          .background(AppTheme.colors.borderSubtle),
      )

      Box(
        modifier = Modifier
          .fillMaxWidth()
          .clickable { onConfirm() }
          .padding(vertical = 14.dp),
        contentAlignment = Alignment.Center,
      ) {
        Text(confirmText, style = AppTheme.typography.action)
      }
    }
  }
}
