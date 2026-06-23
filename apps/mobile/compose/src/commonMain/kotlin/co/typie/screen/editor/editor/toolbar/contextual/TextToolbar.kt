package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import co.typie.editor.EditorColorOption
import co.typie.editor.EditorTheme
import co.typie.editor.EditorValues
import co.typie.editor.ResolvedEditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.ModifierType
import co.typie.graphql.fragment.EditorSettingsFontFamily_family
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarDivider
import co.typie.screen.editor.editor.toolbar.EditorToolbarLabelButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

@Composable
internal fun rememberTextToolbarPage(
  modifierState: ModifierState?,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  activeTextOptionMode: TextOptionMode?,
  onTextOptionModeChange: (TextOptionMode?) -> Unit,
): EditorToolbarPage {
  val scrollState = rememberScrollState()
  return remember(
    scrollState,
    modifierState,
    fontFamilies,
    activeTextOptionMode,
    onTextOptionModeChange,
  ) {
    EditorToolbarPage(
      key = EditorToolbarPageKey.Text,
      icon = Lucide.Type,
      contentDescription = "텍스트 툴바",
      scrollState = scrollState,
      content = { scope ->
        EditorTextToolbar(
          scope = scope,
          scrollState = scrollState,
          modifierState = modifierState,
          fontFamilies = fontFamilies,
          activeTextOptionMode = activeTextOptionMode,
          onTextOptionModeChange = onTextOptionModeChange,
        )
      },
    )
  }
}

@Composable
private fun EditorTextToolbar(
  scope: EditorToolbarPageScope,
  scrollState: ScrollState,
  modifierState: ModifierState?,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  activeTextOptionMode: TextOptionMode?,
  onTextOptionModeChange: (TextOptionMode?) -> Unit,
  modifier: Modifier = Modifier,
) {
  val variant = currentEditorThemeVariant()
  val editorTheme = remember(variant) { EditorTheme.resolve(variant) }
  val textColor = modifierState?.textColor.uniformValue { it.value }
  val backgroundColor = modifierState?.backgroundColor.uniformValue { it.value }
  val fontFamily = modifierState?.fontFamily.uniformValue { it.value }
  val fontWeight = modifierState?.fontWeight.uniformValue { it.value }
  val fontSize = modifierState?.fontSize.uniformValue { it.value }
  val alignment = modifierState?.alignment.uniformValue { it.value }

  fun toggleMode(mode: TextOptionMode) {
    onTextOptionModeChange(if (activeTextOptionMode == mode) null else mode)
  }

  EditorToolbarRow(scope = scope, modifier = modifier, scrollState = scrollState) {
    TextToolbarSwatchButton(
      color = editorTheme.colorFor(EditorValues.textColor, textColor),
      contentDescription = "글자색",
      onClick = { toggleMode(TextOptionMode.TextColor) },
      selected = activeTextOptionMode == TextOptionMode.TextColor,
    )
    TextToolbarSwatchButton(
      color = editorTheme.colorFor(EditorValues.textBackgroundColor, backgroundColor),
      contentDescription = "배경색",
      onClick = { toggleMode(TextOptionMode.BackgroundColor) },
      selected = activeTextOptionMode == TextOptionMode.BackgroundColor,
      swatchShape = TextBackgroundSwatchShape,
      showSlash = backgroundColor == "none",
    )
    EditorToolbarLabelButton(
      text = fontFamilyLabel(fontFamily, fontFamilies),
      contentDescription = "폰트 패밀리",
      onClick = { toggleMode(TextOptionMode.FontFamily) },
      selected = activeTextOptionMode == TextOptionMode.FontFamily,
      subtle = true,
    )
    EditorToolbarLabelButton(
      text = fontWeight?.let { toolbarFontWeightLabel(it) } ?: "-",
      contentDescription = "폰트 굵기",
      onClick = { toggleMode(TextOptionMode.FontWeight) },
      selected = activeTextOptionMode == TextOptionMode.FontWeight,
      subtle = true,
    )
    EditorToolbarLabelButton(
      text = fontSize?.let(::formatToolbarPointValue) ?: "-",
      contentDescription = "폰트 크기",
      onClick = { toggleMode(TextOptionMode.FontSize) },
      selected = activeTextOptionMode == TextOptionMode.FontSize,
      subtle = true,
    )
    EditorToolbarDivider()
    EditorToolbarButton(
      icon = Lucide.Bold,
      contentDescription = "굵게",
      onClick = { scope.sendMessage(Message.Modifier(ModifierOp.Toggle(ModifierType.Bold))) },
      selected = modifierState?.effectiveBold.hasUniformValue(),
    )
    EditorToolbarButton(
      icon = Lucide.Italic,
      contentDescription = "기울임",
      onClick = { scope.sendMessage(Message.Modifier(ModifierOp.Toggle(ModifierType.Italic))) },
      selected = modifierState?.italic.hasUniformValue(),
    )
    EditorToolbarButton(
      icon = Lucide.Underline,
      contentDescription = "밑줄",
      onClick = { scope.sendMessage(Message.Modifier(ModifierOp.Toggle(ModifierType.Underline))) },
      selected = modifierState?.underline.hasUniformValue(),
    )
    EditorToolbarButton(
      icon = Lucide.Strikethrough,
      contentDescription = "취소선",
      onClick = {
        scope.sendMessage(Message.Modifier(ModifierOp.Toggle(ModifierType.Strikethrough)))
      },
      selected = modifierState?.strikethrough.hasUniformValue(),
    )
    EditorToolbarDivider()
    EditorToolbarButton(icon = Lucide.Link, contentDescription = "링크", onClick = {})
    EditorToolbarButton(icon = Typie.Ruby, contentDescription = "루비", onClick = {})
    EditorToolbarDivider()
    EditorToolbarButton(
      icon = toolbarAlignmentIcon(alignment),
      contentDescription = "문단 정렬",
      onClick = { toggleMode(TextOptionMode.Alignment) },
      selected = activeTextOptionMode == TextOptionMode.Alignment,
    )
    EditorToolbarButton(
      icon = Typie.LineHeight,
      contentDescription = "줄 높이",
      onClick = { toggleMode(TextOptionMode.LineHeight) },
      selected = activeTextOptionMode == TextOptionMode.LineHeight,
    )
    EditorToolbarButton(
      icon = Typie.LetterSpacing,
      contentDescription = "자간",
      onClick = { toggleMode(TextOptionMode.LetterSpacing) },
      selected = activeTextOptionMode == TextOptionMode.LetterSpacing,
    )
    EditorToolbarDivider()
    EditorToolbarButton(
      icon = Lucide.RemoveFormatting,
      contentDescription = "서식 지우기",
      onClick = { scope.sendMessage(Message.Modifier(ModifierOp.ClearAll)) },
    )
  }
}

private fun ResolvedEditorTheme.colorFor(options: List<EditorColorOption>, value: String?) =
  value?.let { colorValue ->
    options.firstOrNull { it.value == colorValue }?.themeKey?.let { this[it] }
  }

private fun fontFamilyLabel(
  familyName: String?,
  fontFamilies: List<EditorSettingsFontFamily_family>,
): String {
  if (familyName == null) {
    return "-"
  }
  return fontFamilies.firstOrNull { it.familyName == familyName }?.displayName ?: familyName
}
