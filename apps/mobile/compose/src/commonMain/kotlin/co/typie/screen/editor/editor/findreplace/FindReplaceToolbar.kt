package co.typie.screen.editor.editor.findreplace

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.rememberTextInputState
import co.typie.ext.textInputFocusable
import co.typie.icons.Lucide
import co.typie.platform.PlatformModule
import co.typie.screen.editor.editor.EditorScreenShortcutModifier
import co.typie.screen.editor.editor.matchesEditorShortcut
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarSurfaceBackground
import co.typie.screen.editor.editor.toolbar.ToolbarBorderWidth
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPadding
import co.typie.screen.editor.editor.toolbar.ToolbarButtonSize
import co.typie.screen.editor.editor.toolbar.ToolbarCapsuleShape
import co.typie.screen.editor.editor.toolbar.ToolbarHeight
import co.typie.screen.editor.editor.toolbar.ToolbarHorizontalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarLabelTextStyle
import co.typie.screen.editor.editor.toolbar.ToolbarPageEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageStartPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarVisibilityEnterMillis
import co.typie.screen.editor.editor.toolbar.ToolbarVisibilityExitMillis
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
internal fun FindReplaceToolbar(
  session: EditorFindReplaceSession,
  visibleState: MutableTransitionState<Boolean>,
  bottomInset: Dp,
  modifier: Modifier = Modifier,
  onEscape: () -> Unit = {},
) {
  AnimatedVisibility(
    visibleState = visibleState,
    enter = fadeIn(animationSpec = tween(ToolbarVisibilityEnterMillis)),
    exit = fadeOut(animationSpec = tween(ToolbarVisibilityExitMillis)),
    modifier =
      modifier
        .fillMaxWidth()
        .offset { IntOffset(x = 0, y = -bottomInset.roundToPx()) }
        .padding(
          start = ToolbarHorizontalPadding,
          end = ToolbarHorizontalPadding,
          bottom = ToolbarBottomPadding,
        ),
  ) {
    Box(contentAlignment = Alignment.BottomCenter) {
      Box(
        modifier =
          Modifier.widthIn(max = ResponsiveContainerDefaults.MaxWidth)
            .fillMaxWidth()
            .height(ToolbarHeight)
            .shadow(AppTheme.shadows.sm, ToolbarCapsuleShape)
            .clip(ToolbarCapsuleShape)
            .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarCapsuleShape)
      ) {
        EditorToolbarSurfaceBackground(shape = ToolbarCapsuleShape)
        Row(
          modifier =
            Modifier.fillMaxWidth()
              .height(ToolbarHeight)
              .padding(
                start = ToolbarPageStartPadding,
                end = ToolbarPageEndPadding,
                top = ToolbarPageVerticalPadding,
                bottom = ToolbarPageVerticalPadding,
              ),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          ReplaceTextField(session = session, modifier = Modifier.weight(1f), onEscape = onEscape)
          Spacer(Modifier.width(ToolbarItemGap))
          EditorToolbarButton(
            icon = Lucide.Replace,
            contentDescription = "바꾸기",
            onClick = session.replace,
            enabled = session.canReplace,
          )
          Spacer(Modifier.width(ToolbarItemGap))
          EditorToolbarButton(
            icon = Lucide.ReplaceAll,
            contentDescription = "모두 바꾸기",
            onClick = session.replaceAll,
            enabled = session.canReplace,
          )
          Spacer(Modifier.width(ToolbarItemGap))
          EditorToolbarButton(
            icon = Lucide.ChevronUp,
            contentDescription = "이전 검색 결과",
            onClick = session.findPrevious,
            enabled = session.hasMatches,
          )
          Spacer(Modifier.width(ToolbarItemGap))
          EditorToolbarButton(
            icon = Lucide.ChevronDown,
            contentDescription = "다음 검색 결과",
            onClick = session.findNext,
            enabled = session.hasMatches,
          )
        }
      }
    }
  }
}

@Composable
private fun ReplaceTextField(
  session: EditorFindReplaceSession,
  modifier: Modifier = Modifier,
  onEscape: () -> Unit = {},
) {
  val inputState =
    rememberTextInputState(
      value = session.replaceText,
      onValueChange = session.updateReplaceText,
      onDismiss = {},
    )
  val shape = AppShapes.rounded(AppShapes.full)

  BasicTextField(
    value = inputState.value,
    onValueChange = inputState::onValueChange,
    singleLine = true,
    textStyle = ToolbarLabelTextStyle.copy(color = AppTheme.colors.textDefault),
    cursorBrush = SolidColor(AppTheme.colors.textDefault),
    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
    keyboardActions = KeyboardActions(onDone = { session.replace() }),
    modifier =
      modifier
        .height(ToolbarButtonSize)
        .clip(shape)
        .background(AppTheme.colors.surfaceInset, shape)
        .padding(horizontal = 10.dp)
        .textInputFocusable(inputState)
        .onPreviewKeyEvent { event -> handleReplaceInputShortcut(event, session, onEscape) },
    decorationBox = { innerTextField ->
      Box(contentAlignment = Alignment.CenterStart) {
        if (session.replaceText.isEmpty()) {
          Text(
            text = "바꾸기",
            style = ToolbarLabelTextStyle,
            color = AppTheme.colors.textHint,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
        innerTextField()
      }
    },
  )
}

private fun handleReplaceInputShortcut(
  event: KeyEvent,
  session: EditorFindReplaceSession,
  onEscape: () -> Unit,
): Boolean =
  when {
    matchesEditorShortcut(event = event, platform = PlatformModule.platform, key = Key.Escape) -> {
      onEscape()
      true
    }
    matchesEditorShortcut(
      event = event,
      platform = PlatformModule.platform,
      key = Key.Enter,
      modifiers = setOf(EditorScreenShortcutModifier.Mod),
    ) -> {
      session.replaceAll()
      true
    }
    matchesEditorShortcut(event = event, platform = PlatformModule.platform, key = Key.Enter) -> {
      session.replace()
      true
    }
    else -> false
  }
