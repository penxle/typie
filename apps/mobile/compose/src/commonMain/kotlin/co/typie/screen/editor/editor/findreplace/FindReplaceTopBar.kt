package co.typie.screen.editor.editor.findreplace

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.rememberTextInputState
import co.typie.ext.textInputFocusable
import co.typie.icons.Lucide
import co.typie.platform.PlatformModule
import co.typie.screen.editor.editor.EditorScreenShortcutModifier
import co.typie.screen.editor.editor.matchesEditorShortcut
import co.typie.ui.component.Text
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
internal fun FindReplaceTopBarLeading(session: EditorFindReplaceSession) {
  TopBarButton(
    icon = Lucide.X,
    onClick = { session.close() },
    modifier =
      Modifier.semantics {
        contentDescription = "검색 종료"
        role = Role.Button
      },
  )
}

@Composable
internal fun FindReplaceTopBarCenter(session: EditorFindReplaceSession) {
  val inputState =
    rememberTextInputState(
      value = session.findText,
      onValueChange = session.updateFindText,
      onDismiss = {},
    )

  LaunchedEffect(session.active, session.searchInputFocusRequest) {
    if (session.active) {
      inputState.requestFocus()
    }
  }

  val shape = AppShapes.rounded(AppShapes.full)
  Row(
    modifier =
      Modifier.fillMaxWidth()
        .height(TopBarDefaults.TitleHeight)
        .shadow(AppTheme.shadows.sm, shape)
        .clip(shape)
        .background(TopBarDefaults.controlBackgroundColor(), shape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), shape)
        .padding(horizontal = 14.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = Lucide.Search,
      contentDescription = null,
      modifier = Modifier.size(TopBarDefaults.TitleIconSize),
      tint = AppTheme.colors.textHint,
    )
    Spacer(Modifier.width(TopBarDefaults.TitleIconGap))
    BasicTextField(
      value = inputState.value,
      onValueChange = inputState::onValueChange,
      singleLine = true,
      textStyle = AppTheme.typography.body.copy(color = AppTheme.colors.textDefault),
      cursorBrush = SolidColor(AppTheme.colors.textDefault),
      keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
      keyboardActions = KeyboardActions(onSearch = { session.findNext() }),
      modifier =
        Modifier.weight(1f).textInputFocusable(inputState).onPreviewKeyEvent { event ->
          handleFindInputShortcut(event, session)
        },
      decorationBox = { innerTextField ->
        Box(contentAlignment = Alignment.CenterStart) {
          if (session.findText.isEmpty()) {
            Text(
              text = "찾기",
              style = AppTheme.typography.body,
              color = AppTheme.colors.textHint,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          }
          innerTextField()
        }
      },
    )

    FindReplaceResultLabel(session = session)
  }
}

private fun handleFindInputShortcut(event: KeyEvent, session: EditorFindReplaceSession): Boolean =
  when {
    matchesEditorShortcut(event = event, platform = PlatformModule.platform, key = Key.Escape) -> {
      session.close()
      true
    }
    matchesEditorShortcut(
      event = event,
      platform = PlatformModule.platform,
      key = Key.Enter,
      modifiers = setOf(EditorScreenShortcutModifier.Shift),
    ) -> {
      session.findPrevious()
      true
    }
    matchesEditorShortcut(event = event, platform = PlatformModule.platform, key = Key.Enter) -> {
      session.findNext()
      true
    }
    else -> false
  }

@Composable
internal fun FindReplaceTopBarTrailing(session: EditorFindReplaceSession) {
  PopoverMenu(
    anchor = {
      TopBarButton(
        icon = Lucide.Ellipsis,
        modifier =
          Modifier.semantics {
            contentDescription = "검색 옵션"
            role = Role.Button
          },
      )
    }
  ) {
    item(content = { WholeWordMenuItem(selected = session.matchWholeWord) }) {
      session.updateMatchWholeWord(!session.matchWholeWord)
    }
  }
}

@Composable
private fun FindReplaceResultLabel(session: EditorFindReplaceSession) {
  val label =
    when {
      session.findText.isEmpty() -> null
      session.matchCount == 0 -> "결과 없음"
      else -> "${session.activeMatchNumber ?: 0}/${session.matchCount}"
    }

  if (label == null && !session.matchWholeWord) return

  Spacer(Modifier.width(10.dp))
  Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.widthIn(max = 96.dp)) {
    if (session.matchWholeWord) {
      Icon(
        icon = Lucide.WholeWord,
        contentDescription = null,
        modifier = Modifier.size(14.dp),
        tint = AppTheme.colors.textMuted,
      )
      Spacer(Modifier.width(4.dp))
    }
    if (label != null) {
      Text(
        text = label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textMuted,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier.widthIn(max = 72.dp),
      )
    }
  }
}

@Composable
private fun WholeWordMenuItem(selected: Boolean) {
  Row(
    modifier = Modifier.fillMaxWidth().height(42.dp).padding(horizontal = 16.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = Lucide.WholeWord,
      modifier = Modifier.size(18.dp),
      tint = AppTheme.colors.textDefault,
    )
    Spacer(Modifier.width(12.dp))
    Text(
      text = "어절 단위 검색",
      style = AppTheme.typography.action,
      color = AppTheme.colors.textDefault,
      modifier = Modifier.weight(1f),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
    Box(modifier = Modifier.width(28.dp), contentAlignment = Alignment.CenterEnd) {
      if (selected) {
        Icon(
          icon = Lucide.Check,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textDefault,
        )
      }
    }
  }
}
