package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.editor.EditorColorOption
import co.typie.editor.EditorOption
import co.typie.editor.EditorState
import co.typie.editor.EditorTheme
import co.typie.editor.EditorValues
import co.typie.editor.ResolvedEditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.Alignment as FfiAlignment
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierState
import co.typie.graphql.fragment.EditorSettingsFontFamily_family
import co.typie.graphql.type.FontState
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarLabelButton
import co.typie.screen.editor.editor.toolbar.ToolbarButtonShape
import co.typie.screen.editor.editor.toolbar.ToolbarButtonSize
import co.typie.screen.editor.editor.toolbar.ToolbarFixedActionWidth
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarLabelHorizontalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarLabelTextStyle
import co.typie.screen.editor.editor.toolbar.ToolbarPageEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarTextOptionsSwitchMillis
import co.typie.ui.component.FontSpecimen
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.DialogActionButton
import co.typie.ui.component.dialog.DialogActionDivider
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.DialogScope
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.dismiss
import co.typie.ui.component.dialog.resolve
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
internal fun TextOptionsToolbar(
  mode: TextOptionMode,
  editorState: EditorState,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  onModeChange: (TextOptionMode?) -> Unit,
  sendMessages: (List<Message>) -> Unit,
  runToolbarModal: (suspend () -> Unit) -> Unit,
  modifier: Modifier = Modifier,
) {
  val modifierState = editorState.modifierState
  val variant = currentEditorThemeVariant()
  val editorTheme = remember(variant) { EditorTheme.resolve(variant) }

  TextOptionsToolbarSurface(onClose = { onModeChange(null) }, modifier = modifier) {
    AnimatedContent(
      targetState = mode,
      transitionSpec = {
        val direction = if (targetState.ordinal >= initialState.ordinal) 1 else -1
        (slideInHorizontally(
            animationSpec = tween(ToolbarTextOptionsSwitchMillis),
            initialOffsetX = { direction * 10 },
          ) + fadeIn(animationSpec = tween(ToolbarTextOptionsSwitchMillis)))
          .togetherWith(
            slideOutHorizontally(
              animationSpec = tween(ToolbarTextOptionsSwitchMillis),
              targetOffsetX = { -direction * 10 },
            ) + fadeOut(animationSpec = tween(ToolbarTextOptionsSwitchMillis))
          )
          .using(SizeTransform(clip = false) { _, _ -> tween(ToolbarTextOptionsSwitchMillis) })
      },
      contentAlignment = Alignment.CenterStart,
      label = "TextOptionsToolbarMode",
    ) { targetMode ->
      Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
      ) {
        TextOptionsContent(
          mode = targetMode,
          modifierState = modifierState,
          editorTheme = editorTheme,
          fontFamilies = fontFamilies,
          sendMessages = sendMessages,
          runToolbarModal = runToolbarModal,
        )
      }
    }
  }
}

@Composable
private fun TextOptionsContent(
  mode: TextOptionMode,
  modifierState: ModifierState?,
  editorTheme: ResolvedEditorTheme,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  sendMessages: (List<Message>) -> Unit,
  runToolbarModal: (suspend () -> Unit) -> Unit,
) {
  when (mode) {
    TextOptionMode.TextColor ->
      ColorOptions(
        options = EditorValues.textColor,
        currentValue = modifierState?.textColor.uniformValue { it.value },
        editorTheme = editorTheme,
        swatchShape = ToolbarButtonShape,
        showSlashForNullTheme = false,
        onSelect = { sendSet(sendMessages, EditorModifier.TextColor(it)) },
      )
    TextOptionMode.BackgroundColor ->
      TextBackgroundColorOptions(
        currentValue = modifierState?.backgroundColor.uniformValue { it.value },
        editorTheme = editorTheme,
        onSelect = { sendSet(sendMessages, EditorModifier.BackgroundColor(it)) },
      )
    TextOptionMode.FontFamily ->
      FontFamilyOptions(
        fontFamilies = fontFamilies,
        modifierState = modifierState,
        sendMessages = sendMessages,
      )
    TextOptionMode.FontWeight ->
      FontWeightOptions(
        fontFamilies = fontFamilies,
        modifierState = modifierState,
        onSelect = { sendSet(sendMessages, EditorModifier.FontWeight(it)) },
      )
    TextOptionMode.FontSize ->
      FontSizeOptions(
        currentValue = modifierState?.fontSize.uniformValue { it.value },
        onSelect = { sendSet(sendMessages, EditorModifier.FontSize(it)) },
        runToolbarModal = runToolbarModal,
      )
    TextOptionMode.Alignment ->
      AlignmentOptions(
        currentValue = modifierState?.alignment.uniformValue { it.value },
        onSelect = { sendSet(sendMessages, EditorModifier.Alignment(it)) },
      )
    TextOptionMode.LineHeight ->
      PercentOptions(
        options = EditorValues.lineHeight,
        currentValue = modifierState?.lineHeight.uniformValue { it.value },
        contentDescriptionPrefix = "줄 높이",
        onSelect = { sendSet(sendMessages, EditorModifier.LineHeight(it)) },
      )
    TextOptionMode.LetterSpacing ->
      PercentOptions(
        options = EditorValues.letterSpacing,
        currentValue = modifierState?.letterSpacing.uniformValue { it.value },
        contentDescriptionPrefix = "자간",
        onSelect = { sendSet(sendMessages, EditorModifier.LetterSpacing(it)) },
      )
  }
}

@Composable
private fun TextOptionsToolbarSurface(
  onClose: () -> Unit,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  val scrollState = rememberScrollState()
  val scrollStartPadding = ToolbarFixedActionWidth

  ToolbarSecondarySurface(
    onClose = onClose,
    closeContentDescription = "텍스트 옵션 닫기",
    modifier = modifier,
  ) {
    Row(
      modifier =
        Modifier.fillMaxSize()
          .horizontalScroll(scrollState)
          .padding(
            start = scrollStartPadding,
            top = ToolbarPageVerticalPadding,
            end = ToolbarPageEndPadding,
            bottom = ToolbarPageVerticalPadding,
          ),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
    ) {
      content()
    }
  }
}

@Composable
internal fun TextBackgroundColorOptions(
  currentValue: String?,
  editorTheme: ResolvedEditorTheme,
  onSelect: (String) -> Unit,
) {
  ColorOptions(
    options = EditorValues.textBackgroundColor,
    currentValue = currentValue,
    editorTheme = editorTheme,
    swatchShape = TextBackgroundSwatchShape,
    showSlashForNullTheme = true,
    onSelect = onSelect,
  )
}

@Composable
private fun ColorOptions(
  options: List<EditorColorOption>,
  currentValue: String?,
  editorTheme: ResolvedEditorTheme,
  swatchShape: Shape,
  showSlashForNullTheme: Boolean,
  onSelect: (String) -> Unit,
) {
  options.forEach { option ->
    TextToolbarSwatchButton(
      color = option.themeKey?.let { editorTheme[it] },
      contentDescription = option.label,
      onClick = { onSelect(option.value) },
      selected = currentValue == option.value,
      swatchShape = swatchShape,
      showSlash = showSlashForNullTheme && option.themeKey == null,
    )
  }
}

@Composable
private fun FontFamilyOptions(
  fontFamilies: List<EditorSettingsFontFamily_family>,
  modifierState: ModifierState?,
  sendMessages: (List<Message>) -> Unit,
) {
  val currentFamilyName = modifierState?.fontFamily.uniformValue { it.value }
  val selectableFamilies = fontFamilies.filter { it.isSelectableToolbarFontFamily }
  val currentFamily = fontFamilies.firstOrNull { it.familyName == currentFamilyName }
  val families =
    remember(selectableFamilies, currentFamily, currentFamilyName) {
      buildList {
        if (
          currentFamily != null && selectableFamilies.none { it.familyName == currentFamilyName }
        ) {
          add(currentFamily)
        }
        addAll(selectableFamilies)
      }
    }

  families.forEach { family ->
    val representativeFont = family.representativeToolbarFont
    val selected = family.familyName == currentFamilyName
    TextOptionContentButton(
      selected = selected,
      contentDescription = family.displayName,
      onClick = { sendSet(sendMessages, EditorModifier.FontFamily(family.familyName)) },
    ) { contentColor ->
      if (representativeFont != null) {
        FontSpecimen(
          fontId = representativeFont.id,
          text = family.displayName,
          fallbackTexts = listOf(family.familyName),
          style =
            TextStyle(
              color = contentColor,
              fontSize = 16.sp,
              fontWeight = FontWeight(representativeFont.weight),
            ),
        )
      } else {
        Text(
          text = family.displayName,
          style = ToolbarLabelTextStyle,
          color = contentColor,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }
  }
}

@Composable
private fun FontWeightOptions(
  fontFamilies: List<EditorSettingsFontFamily_family>,
  modifierState: ModifierState?,
  onSelect: (Int) -> Unit,
) {
  val currentFamilyName = modifierState?.fontFamily.uniformValue { it.value }
  val currentWeight = modifierState?.fontWeight.uniformValue { it.value }
  val family = fontFamilies.firstOrNull { it.familyName == currentFamilyName }
  val fontOptions =
    if (family != null) {
      family.fonts
        .filter { it.state == FontState.ACTIVE }
        .distinctBy { it.weight }
        .sortedBy { it.weight }
    } else {
      fontFamilies.flatMap { it.activeToolbarFonts }.distinctBy { it.weight }.sortedBy { it.weight }
    }

  if (fontOptions.isNotEmpty()) {
    fontOptions.forEach { font ->
      val selected = currentWeight == font.weight
      TextOptionContentButton(
        selected = selected,
        contentDescription = toolbarFontWeightLabel(font.weight, font.subfamilyDisplayName),
        onClick = { onSelect(font.weight) },
      ) { contentColor ->
        FontSpecimen(
          fontId = font.id,
          text = toolbarFontWeightLabel(font.weight, font.subfamilyDisplayName),
          style =
            TextStyle(color = contentColor, fontSize = 16.sp, fontWeight = FontWeight(font.weight)),
        )
      }
    }
  } else {
    EditorValues.fontWeight.forEach { option ->
      EditorToolbarLabelButton(
        text = option.label,
        contentDescription = option.label,
        onClick = { onSelect(option.value) },
        selected = currentWeight == option.value,
        autoBringIntoView = currentWeight == option.value,
        subtle = true,
      )
    }
  }
}

@Composable
private fun FontSizeOptions(
  currentValue: Int?,
  onSelect: (Int) -> Unit,
  runToolbarModal: (suspend () -> Unit) -> Unit,
) {
  val dialog = LocalDialog.current

  fun openDirectInput(initialValue: Int) {
    runToolbarModal {
      withFrameNanos {}
      val result =
        dialog.present<Int>(dismissible = true) { FontSizeInputDialog(initialValue = initialValue) }
      if (result is DialogResult.Resolved) {
        onSelect(result.value)
      }
    }
  }

  val options = EditorValues.fontSize.withCurrent(currentValue) { formatToolbarPointValue(it) }
  options.forEach { option ->
    val selected = currentValue == option.value
    EditorToolbarLabelButton(
      text = option.label,
      contentDescription = "폰트 크기 ${option.label}",
      onClick = {
        if (selected) {
          openDirectInput(option.value)
        } else {
          onSelect(option.value)
        }
      },
      selected = selected,
      suffixIcon = if (selected) Lucide.Pencil else null,
      autoBringIntoView = selected,
      subtle = true,
    )
  }
}

@Composable
private fun AlignmentOptions(currentValue: FfiAlignment?, onSelect: (FfiAlignment) -> Unit) {
  listOf(FfiAlignment.Left, FfiAlignment.Center, FfiAlignment.Right, FfiAlignment.Justify)
    .forEach { alignment ->
      EditorToolbarLabelButton(
        text = toolbarAlignmentLabel(alignment),
        contentDescription = "${toolbarAlignmentLabel(alignment)} 정렬",
        onClick = { onSelect(alignment) },
        selected = currentValue == alignment,
        autoBringIntoView = currentValue == alignment,
        subtle = true,
      )
    }
}

@Composable
private fun PercentOptions(
  options: List<EditorOption<Int>>,
  currentValue: Int?,
  contentDescriptionPrefix: String,
  onSelect: (Int) -> Unit,
) {
  options
    .withCurrent(currentValue) { "$it%" }
    .forEach { option ->
      EditorToolbarLabelButton(
        text = option.label,
        contentDescription = "$contentDescriptionPrefix ${option.label}",
        onClick = { onSelect(option.value) },
        selected = currentValue == option.value,
        autoBringIntoView = currentValue == option.value,
        subtle = true,
      )
    }
}

@Composable
private fun TextOptionContentButton(
  selected: Boolean,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  content: @Composable (Color) -> Unit,
) {
  val interactionSource = remember { MutableInteractionSource() }
  val bringIntoViewRequester = remember { BringIntoViewRequester() }
  val contentColor = if (selected) AppTheme.colors.textDefault else AppTheme.colors.textHint

  LaunchedEffect(selected) {
    if (selected) {
      bringIntoViewRequester.bringIntoView()
    }
  }

  Box(
    modifier =
      modifier
        .height(ToolbarButtonSize)
        .bringIntoViewRequester(bringIntoViewRequester)
        .focusProperties { canFocus = false }
        .semantics {
          this.contentDescription = contentDescription
          role = Role.Button
        }
        .clip(ToolbarButtonShape)
        .then(
          if (selected) {
            Modifier.background(AppTheme.colors.surfaceInset, ToolbarButtonShape)
          } else {
            Modifier
          }
        )
        .clickable(interactionSource = interactionSource, indication = null, onClick = onClick)
        .padding(horizontal = ToolbarLabelHorizontalPadding),
    contentAlignment = Alignment.Center,
  ) {
    content(contentColor)
  }
}

@Composable
context(scope: DialogScope<Int>)
private fun FontSizeInputDialog(initialValue: Int) {
  var value by remember(initialValue) { mutableStateOf(formatToolbarPointValue(initialValue)) }
  val parsedValue = remember(value) { parseFontSizeInput(value) }
  val hasError = value.isNotBlank() && parsedValue == null

  fun submit() {
    parsedValue?.let { resolve(it) }
  }

  Column(
    modifier =
      Modifier.widthIn(max = 340.dp)
        .clip(AppShapes.rounded(AppShapes.lg))
        .background(AppTheme.colors.surfaceDefault)
  ) {
    Column(Modifier.padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 20.dp)) {
      Text("폰트 크기", style = AppTheme.typography.title)
      Spacer(Modifier.height(16.dp))
      TextField(
        value = value,
        onValueChange = { value = it },
        label = "크기",
        labelPosition = LabelPosition.None,
        autoFocus = true,
        placeholder =
          "${formatToolbarPointValue(EditorValues.minFontSize)}-" +
            formatToolbarPointValue(EditorValues.maxFontSize),
        keyboardType = KeyboardType.Decimal,
        imeAction = ImeAction.Done,
        onImeAction = ::submit,
        error = if (hasError) "1-200 사이 숫자를 입력하세요" else null,
        suffix = {
          Text("pt", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)
        },
        modifier = Modifier.fillMaxWidth(),
      )
    }

    Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))

    Row(Modifier.fillMaxWidth()) {
      DialogActionButton(text = "취소") { dismiss() }
      DialogActionDivider()
      DialogActionButton(text = "확인") { submit() }
    }
  }
}

private fun sendSet(sendMessages: (List<Message>) -> Unit, modifier: EditorModifier) {
  sendMessages(listOf(Message.Modifier(ModifierOp.Set(modifier))))
}

private fun List<EditorOption<Int>>.withCurrent(
  currentValue: Int?,
  label: (Int) -> String,
): List<EditorOption<Int>> =
  if (currentValue == null || any { it.value == currentValue }) {
    this
  } else {
    (this + EditorOption(label = label(currentValue), value = currentValue)).sortedBy { it.value }
  }

private fun parseFontSizeInput(value: String): Int? {
  val parsed = value.trim().toDoubleOrNull()?.let { (it * 100).roundToInt() } ?: return null
  return parsed.takeIf { it in EditorValues.minFontSize..EditorValues.maxFontSize }
}
