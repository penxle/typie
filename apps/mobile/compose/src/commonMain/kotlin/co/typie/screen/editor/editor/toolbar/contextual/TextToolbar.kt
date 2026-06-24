package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorColorOption
import co.typie.editor.EditorTheme
import co.typie.editor.EditorValues
import co.typie.editor.ResolvedEditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Tri
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
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogActionButton
import co.typie.ui.component.dialog.DialogActionDivider
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.DialogScope
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.dismiss
import co.typie.ui.component.dialog.resolve
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
internal fun rememberTextToolbarPage(
  modifierState: ModifierState?,
  selection: Selection?,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  activeTextOptionMode: TextOptionMode?,
  onTextOptionModeChange: (TextOptionMode?) -> Unit,
  runToolbarModal: (suspend () -> Unit) -> Unit,
): EditorToolbarPage {
  val scrollState = rememberScrollState()
  return remember(
    scrollState,
    modifierState,
    selection,
    fontFamilies,
    activeTextOptionMode,
    onTextOptionModeChange,
    runToolbarModal,
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
          selection = selection,
          fontFamilies = fontFamilies,
          activeTextOptionMode = activeTextOptionMode,
          onTextOptionModeChange = onTextOptionModeChange,
          runToolbarModal = runToolbarModal,
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
  selection: Selection?,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  activeTextOptionMode: TextOptionMode?,
  onTextOptionModeChange: (TextOptionMode?) -> Unit,
  runToolbarModal: (suspend () -> Unit) -> Unit,
  modifier: Modifier = Modifier,
) {
  val dialog = LocalDialog.current
  val variant = currentEditorThemeVariant()
  val editorTheme = remember(variant) { EditorTheme.resolve(variant) }
  val textColor = modifierState?.textColor.uniformValue { it.value }
  val backgroundColor = modifierState?.backgroundColor.uniformValue { it.value }
  val fontFamily = modifierState?.fontFamily.uniformValue { it.value }
  val fontWeight = modifierState?.fontWeight.uniformValue { it.value }
  val fontSize = modifierState?.fontSize.uniformValue { it.value }
  val alignment = modifierState?.alignment.uniformValue { it.value }
  val selectionCollapsed = selection == null || selection.anchor == selection.head
  val link = modifierState?.link
  val linkHref = (link as? Tri.Uniform)?.value?.href
  val linkActive = linkHref != null
  val linkEnabled = link !is Tri.Mixed && (!selectionCollapsed || linkActive)
  val ruby = modifierState?.ruby
  val rubyText = (ruby as? Tri.Uniform)?.value?.text
  val rubyActive = rubyText != null
  val rubyEnabled = ruby !is Tri.Mixed && (!selectionCollapsed || rubyActive)

  fun toggleMode(mode: TextOptionMode) {
    onTextOptionModeChange(if (activeTextOptionMode == mode) null else mode)
  }

  fun openLinkInput() {
    if (!linkEnabled) return
    onTextOptionModeChange(null)
    runToolbarModal {
      withFrameNanos {}
      val message = dialog.promptLinkMessage(linkHref)
      if (message != null) {
        scope.sendMessage(message)
      }
    }
  }

  fun openRubyInput() {
    if (!rubyEnabled) return
    onTextOptionModeChange(null)
    runToolbarModal {
      withFrameNanos {}
      val message = dialog.promptRubyMessage(rubyText)
      if (message != null) {
        scope.sendMessage(message)
      }
    }
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
    EditorToolbarButton(
      icon = Lucide.Link,
      contentDescription = "링크",
      onClick = ::openLinkInput,
      selected = linkActive,
      enabled = linkEnabled,
    )
    EditorToolbarButton(
      icon = Typie.Ruby,
      contentDescription = "루비",
      onClick = ::openRubyInput,
      selected = rubyActive,
      enabled = rubyEnabled,
    )
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

private suspend fun Dialog.promptLinkMessage(existingHref: String?): Message? {
  return when (
    val result = present<String?>(dismissible = true) { LinkInputDialog(existingHref) }
  ) {
    is DialogResult.Resolved -> {
      val href = result.value?.trim()
      when {
        href == null ->
          Message.Modifier(ModifierOp.Edit(modifierType = ModifierType.Link, modifier = null))

        href.isEmpty() -> null

        else ->
          Message.Modifier(
            ModifierOp.Edit(
              modifierType = ModifierType.Link,
              modifier = EditorModifier.Link(normalizeLinkUrl(href)),
            )
          )
      }
    }
    DialogResult.Dismissed -> null
  }
}

private suspend fun Dialog.promptRubyMessage(existingText: String?): Message? {
  return when (
    val result = present<String?>(dismissible = true) { RubyInputDialog(existingText) }
  ) {
    is DialogResult.Resolved -> {
      val text = result.value
      when {
        text == null ->
          Message.Modifier(ModifierOp.Edit(modifierType = ModifierType.Ruby, modifier = null))

        text.isEmpty() -> null

        else ->
          Message.Modifier(
            ModifierOp.Edit(modifierType = ModifierType.Ruby, modifier = EditorModifier.Ruby(text))
          )
      }
    }
    DialogResult.Dismissed -> null
  }
}

private fun normalizeLinkUrl(input: String): String =
  if (LinkProtocolRegex.containsMatchIn(input)) input else "https://$input"

private val LinkProtocolRegex = Regex("^(https?:|mailto:|tel:)", RegexOption.IGNORE_CASE)

@Composable
context(scope: DialogScope<String?>)
private fun LinkInputDialog(existingHref: String?) {
  var value by remember(existingHref) { mutableStateOf(existingHref.orEmpty()) }

  fun submit() {
    val href = value.trim()
    if (href.isNotEmpty()) {
      resolve(href)
    }
  }

  Column(
    modifier =
      Modifier.widthIn(max = 340.dp)
        .clip(AppShapes.rounded(AppShapes.lg))
        .background(AppTheme.colors.surfaceDefault)
  ) {
    Column(Modifier.padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 20.dp)) {
      Text("링크", style = AppTheme.typography.title)
      Spacer(Modifier.height(16.dp))
      TextField(
        value = value,
        onValueChange = { value = it },
        label = "URL",
        labelPosition = LabelPosition.None,
        autoFocus = true,
        placeholder = "https://...",
        keyboardType = KeyboardType.Uri,
        imeAction = ImeAction.Done,
        onImeAction = ::submit,
        modifier = Modifier.fillMaxWidth(),
      )
    }

    Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))

    Row(Modifier.fillMaxWidth()) {
      DialogActionButton(text = "취소") { dismiss() }
      if (existingHref != null) {
        DialogActionDivider()
        DialogActionButton(text = "제거", color = AppTheme.colors.danger) { resolve(null) }
      }
      DialogActionDivider()
      DialogActionButton(text = if (existingHref != null) "수정" else "삽입") { submit() }
    }
  }
}

@Composable
context(scope: DialogScope<String?>)
private fun RubyInputDialog(existingText: String?) {
  var value by remember(existingText) { mutableStateOf(existingText.orEmpty()) }

  fun submit() {
    if (value.isNotEmpty()) {
      resolve(value)
    }
  }

  Column(
    modifier =
      Modifier.widthIn(max = 340.dp)
        .clip(AppShapes.rounded(AppShapes.lg))
        .background(AppTheme.colors.surfaceDefault)
  ) {
    Column(Modifier.padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 20.dp)) {
      Text("루비", style = AppTheme.typography.title)
      Spacer(Modifier.height(16.dp))
      TextField(
        value = value,
        onValueChange = { value = it },
        label = "루비",
        labelPosition = LabelPosition.None,
        autoFocus = true,
        placeholder = "텍스트 위에 들어갈 문구",
        keyboardType = KeyboardType.Text,
        imeAction = ImeAction.Done,
        onImeAction = ::submit,
        modifier = Modifier.fillMaxWidth(),
      )
    }

    Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))

    Row(Modifier.fillMaxWidth()) {
      DialogActionButton(text = "취소") { dismiss() }
      if (existingText != null) {
        DialogActionDivider()
        DialogActionButton(text = "제거", color = AppTheme.colors.danger) { resolve(null) }
      }
      DialogActionDivider()
      DialogActionButton(text = if (existingText != null) "수정" else "삽입") { submit() }
    }
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
