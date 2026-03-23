package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import co.typie.ext.clickable
import co.typie.icons.Typie
import co.typie.navigation.Nav
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

@Composable
fun ErrorDialog(
  onRetry: () -> Unit,
) {
  val nav = Nav.current

  Dialog(
    onDismissRequest = {},
    properties = DialogProperties(
      dismissOnBackPress = false,
      dismissOnClickOutside = false,
    ),
  ) {
    Column(
      modifier = Modifier
        .width(280.dp)
        .clip(RoundedCornerShape(16.dp))
        .background(AppTheme.colors.surfaceElevated),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      // Content area
      Column(
        modifier = Modifier.padding(start = 28.dp, end = 28.dp, top = 32.dp, bottom = 28.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        // Error icon
        Box(
          modifier = Modifier
            .size(48.dp)
            .background(AppTheme.colors.accentDangerSubtle, CircleShape),
          contentAlignment = Alignment.Center,
        ) {
          Icon(Typie.ExclamationSvg, tint = AppTheme.colors.accentDanger)
        }
        Spacer(Modifier.height(16.dp))
        Text("문제가 발생했어요", style = AppTheme.typography.title)
        Spacer(Modifier.height(6.dp))
        Text(
          "잠시 후 다시 시도해주세요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textFaint,
        )
      }

      // Separator
      Box(
        Modifier
          .fillMaxWidth()
          .height(1.dp)
          .background(AppTheme.colors.borderSubtle)
      )

      // Button
      Box(
        modifier = Modifier
          .fillMaxWidth()
          .clickable { if (nav.canPop) nav.pop() else onRetry() }
          .padding(vertical = 14.dp),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          if (nav.canPop) "뒤로 가기" else "다시 시도",
          style = AppTheme.typography.action,
        )
      }
    }
  }
}
