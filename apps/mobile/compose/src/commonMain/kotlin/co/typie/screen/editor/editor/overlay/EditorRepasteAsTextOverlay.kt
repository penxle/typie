package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlinx.coroutines.launch

private val RepasteAsTextOverlayShape = AppShapes.rounded(AppShapes.full)
private val RepasteAsTextOverlayTopGap = 12.dp
private val RepasteAsTextOverlayEasing = CubicBezierEasing(0.215f, 0.61f, 0.355f, 1f)
private const val RepasteAsTextOverlayAnimationMillis = 150

@Composable
internal fun EditorRepasteAsTextOverlay(
  editor: Editor,
  visibleArea: EditorVisibleArea,
  visible: Boolean,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  modifier: Modifier = Modifier,
) {
  AnimatedVisibility(
    visible = visible,
    enter =
      fadeIn(
        animationSpec =
          tween(
            durationMillis = RepasteAsTextOverlayAnimationMillis,
            easing = RepasteAsTextOverlayEasing,
          )
      ) +
        scaleIn(
          initialScale = 0.96f,
          transformOrigin = TransformOrigin(0.5f, 0f),
          animationSpec =
            tween(
              durationMillis = RepasteAsTextOverlayAnimationMillis,
              easing = RepasteAsTextOverlayEasing,
            ),
        ),
    exit =
      fadeOut(
        animationSpec =
          tween(
            durationMillis = RepasteAsTextOverlayAnimationMillis,
            easing = RepasteAsTextOverlayEasing,
          )
      ) +
        scaleOut(
          targetScale = 0.96f,
          transformOrigin = TransformOrigin(0.5f, 0f),
          animationSpec =
            tween(
              durationMillis = RepasteAsTextOverlayAnimationMillis,
              easing = RepasteAsTextOverlayEasing,
            ),
        ),
    modifier = modifier.fillMaxSize(),
  ) {
    Box(
      modifier =
        Modifier.fillMaxSize()
          .padding(top = visibleArea.visibleViewportTop.dp + RepasteAsTextOverlayTopGap),
      contentAlignment = Alignment.TopCenter,
    ) {
      Row(
        modifier =
          Modifier.shadow(AppTheme.shadows.sm, RepasteAsTextOverlayShape)
            .clip(RepasteAsTextOverlayShape)
            .border(1.dp, AppTheme.colors.borderDefault, RepasteAsTextOverlayShape)
            .background(AppTheme.colors.surfaceDefault, RepasteAsTextOverlayShape)
            .semantics(mergeDescendants = true) {}
            .clickable {
              editor.trackLocalEdit { context ->
                editor.scope.launch(context) {
                  editor.awaitWithBringIntoView(bringIntoViewRequests) {
                    enqueue(Message.Clipboard(ClipboardOp.RepasteAsText))
                    beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
                  }
                }
              }
              editor.focus()
            }
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Icon(
          icon = Lucide.ClipboardType,
          contentDescription = null,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textMuted,
        )
        Text(
          text = "서식 없이 다시 붙여넣기",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textDefault,
          modifier = Modifier.padding(start = 8.dp),
        )
      }
    }
  }
}
