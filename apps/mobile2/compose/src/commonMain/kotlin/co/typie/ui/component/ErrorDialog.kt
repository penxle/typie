package co.typie.ui.component

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import co.typie.ext.clickable
import co.typie.icons.Typie
import co.typie.navigation.Nav
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

@Composable
fun ErrorDialog(onRetry: () -> Unit) {
  val nav = Nav.current
  var dismissed by remember { mutableStateOf(false) }

  var visible by remember { mutableStateOf(false) }
  LaunchedEffect(Unit) { visible = true }

  if (dismissed) return

  val alpha by
    animateFloatAsState(targetValue = if (visible) 1f else 0f, animationSpec = tween(200))
  val scale by
    animateFloatAsState(targetValue = if (visible) 1f else 0.9f, animationSpec = tween(200))

  Dialog(
    onDismissRequest = {},
    properties = DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false),
  ) {
    Column(
      modifier =
        Modifier.graphicsLayer(alpha = alpha, scaleX = scale, scaleY = scale)
          .width(280.dp)
          .clip(RoundedCornerShape(16.dp))
          .background(AppTheme.colors.surfaceRaised),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      // Content area
      Column(
        modifier = Modifier.padding(start = 28.dp, end = 28.dp, top = 32.dp, bottom = 28.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        // Error icon
        Box(
          modifier = Modifier.size(48.dp).background(AppTheme.colors.dangerSubtle, CircleShape),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            Typie.ExclamationSvg,
            modifier = Modifier.size(20.dp),
            tint = AppTheme.colors.textOnDangerSubtle,
          )
        }
        Spacer(Modifier.height(16.dp))
        Text("문제가 발생했어요", style = AppTheme.typography.title)
        Spacer(Modifier.height(6.dp))
        Text(
          "잠시 후 다시 시도해주세요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )
      }

      // Separator
      Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderSubtle))

      // Button
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .clickable {
              if (nav.canPop) {
                dismissed = true
                nav.pop()
              } else {
                onRetry()
              }
            }
            .padding(vertical = 14.dp),
        contentAlignment = Alignment.Center,
      ) {
        Text(if (nav.canPop) "뒤로 가기" else "다시 시도", style = AppTheme.typography.action)
      }
    }
  }
}
