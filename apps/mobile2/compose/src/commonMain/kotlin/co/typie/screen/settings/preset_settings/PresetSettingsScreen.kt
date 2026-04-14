package co.typie.screen.settings.preset_settings

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.form.FormState
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.Result
import co.typie.result.isOk
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.FontSpecimen
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.familySpecimenFallbacks
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.close
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.Toast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.component.weightSpecimenFallbacks
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

private enum class PresetEditorField {
  FontFamily,
  FontWeight,
  FontSize,
  LetterSpacing,
  LineHeight,
  TextColor,
  BackgroundColor,
  LayoutMode,
  MaxWidth,
  PageSize,
  PageMargin,
  ParagraphIndent,
  BlockGap,
}

private class FontSizeSheetForm(scope: kotlinx.coroutines.CoroutineScope, initialFontSize: Int) :
  FormState(scope) {
  val fontSize = field(formatPresetPointValue(initialFontSize))
}

private class PageSizeSheetForm(
  scope: kotlinx.coroutines.CoroutineScope,
  initialLayout: PresetLayout.Paginated,
) : FormState(scope) {
  val widthMm = field(pxToMm(initialLayout.pageWidth).toString())
  val heightMm = field(pxToMm(initialLayout.pageHeight).toString())
}

private class PageMarginSheetForm(
  scope: kotlinx.coroutines.CoroutineScope,
  initialLayout: PresetLayout.Paginated,
) : FormState(scope) {
  val topMm = field(pxToMm(initialLayout.pageMarginTop).toString())
  val bottomMm = field(pxToMm(initialLayout.pageMarginBottom).toString())
  val leftMm = field(pxToMm(initialLayout.pageMarginLeft).toString())
  val rightMm = field(pxToMm(initialLayout.pageMarginRight).toString())
}

private val PresetSheetPadding =
  SheetPadding(header = PaddingValues(horizontal = 16.dp), body = PaddingValues(horizontal = 16.dp))

@Composable
fun PresetSettingsScreen() {
  val model = viewModel { PresetSettingsViewModel() }
  val nav = Nav.current
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val sheet = LocalSheet.current
  val actionScope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  suspend fun openEditor(field: PresetEditorField) {
    val template = model.currentTemplate
    try {
      when (field) {
        PresetEditorField.FontFamily ->
          sheet.present<Unit> { FontFamilyContent(model = model, template = template) }
        PresetEditorField.FontWeight ->
          sheet.present<Unit> { FontWeightContent(model = model, template = template) }
        PresetEditorField.FontSize ->
          sheet.present<Unit> { FontSizeContent(model = model, template = template) }
        PresetEditorField.LetterSpacing ->
          sheet.present<Unit> {
            PresetOptionContent(
              title = "자간",
              initialValue = model.currentTemplate.letterSpacing,
              options = LETTER_SPACING_OPTIONS,
              onSaveValue = { next ->
                model.saveTemplate(model.currentTemplate.withLetterSpacing(next))
              },
            )
          }
        PresetEditorField.LineHeight ->
          sheet.present<Unit> {
            PresetOptionContent(
              title = "행간",
              initialValue = model.currentTemplate.lineHeight,
              options = LINE_HEIGHT_OPTIONS,
              onSaveValue = { next ->
                model.saveTemplate(model.currentTemplate.withLineHeight(next))
              },
            )
          }
        PresetEditorField.TextColor ->
          sheet.present<Unit> {
            PresetColorContent(
              title = "글자 색",
              initialValue = model.currentTemplate.textColor,
              options = TEXT_COLOR_OPTIONS,
              background = false,
              onSaveValue = { next ->
                model.saveTemplate(model.currentTemplate.withTextColor(next))
              },
            )
          }
        PresetEditorField.BackgroundColor ->
          sheet.present<Unit> {
            PresetColorContent(
              title = "배경 색",
              initialValue = model.currentTemplate.backgroundColor,
              options = BACKGROUND_COLOR_OPTIONS,
              background = true,
              onSaveValue = { next ->
                model.saveTemplate(model.currentTemplate.withBackgroundColor(next))
              },
            )
          }
        PresetEditorField.LayoutMode ->
          sheet.present<Unit> { LayoutModeContent(model = model, template = template) }
        PresetEditorField.MaxWidth ->
          sheet.present<Unit> { MaxWidthContent(model = model, template = template) }
        PresetEditorField.PageSize ->
          sheet.present<Unit> { PageSizeContent(model = model, template = template) }
        PresetEditorField.PageMargin ->
          sheet.present<Unit> { PageMarginContent(model = model, template = template) }
        PresetEditorField.ParagraphIndent ->
          sheet.present<Unit> {
            PresetOptionContent(
              title = "첫 줄 들여쓰기",
              initialValue = model.currentTemplate.paragraphIndent,
              options = PARAGRAPH_INDENT_OPTIONS,
              onSaveValue = { next ->
                model.saveTemplate(model.currentTemplate.withParagraphIndent(next))
              },
            )
          }
        PresetEditorField.BlockGap ->
          sheet.present<Unit> {
            PresetOptionContent(
              title = "문단 간격",
              initialValue = model.currentTemplate.blockGap,
              options = BLOCK_GAP_OPTIONS,
              onSaveValue = { next -> model.saveTemplate(model.currentTemplate.withBlockGap(next)) },
            )
          }
      }
    } catch (_: CancellationException) {
      return
    }
  }

  val template = model.currentTemplate

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("프리셋", style = AppTheme.typography.title) },
    trailing = { PresetTopBarMenu(model = model, actionScope = actionScope) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        text = "프리셋",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      Text(
        text = "새 문서를 생성할 때 자동으로 적용될 기본 포맷을 설정해요.",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )

      PresetSettingsSection(title = "기본 스타일") {
        PresetSettingsRow(
          label = "폰트 패밀리",
          value = fontFamilySummaryLabel(template, model.normalizedFontFamilyOptions),
          onClick = { openEditor(PresetEditorField.FontFamily) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "폰트 굵기",
          value = fontWeightSummaryLabel(template, model.selectedFontWeightAvailability),
          onClick = { openEditor(PresetEditorField.FontWeight) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "폰트 크기",
          value = fontSizeSummaryLabel(template.fontSize),
          onClick = { openEditor(PresetEditorField.FontSize) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "자간",
          value = presetOptionLabel(LETTER_SPACING_OPTIONS, template.letterSpacing),
          onClick = { openEditor(PresetEditorField.LetterSpacing) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "행간",
          value = presetOptionLabel(LINE_HEIGHT_OPTIONS, template.lineHeight),
          onClick = { openEditor(PresetEditorField.LineHeight) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "글자 색",
          value = presetOptionLabel(TEXT_COLOR_OPTIONS, template.textColor, template.textColor),
          onClick = { openEditor(PresetEditorField.TextColor) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "배경 색",
          value =
            presetOptionLabel(
              BACKGROUND_COLOR_OPTIONS,
              template.backgroundColor,
              template.backgroundColor,
            ),
          onClick = { openEditor(PresetEditorField.BackgroundColor) },
        )
      }

      PresetSettingsSection(title = "레이아웃") {
        PresetSettingsRow(
          label = "레이아웃 모드",
          value = layoutModeSummaryLabel(template.layout),
          onClick = { openEditor(PresetEditorField.LayoutMode) },
        )

        when (val layout = template.layout) {
          is PresetLayout.Continuous -> {
            CardDivider()
            PresetSettingsRow(
              label = "본문 폭",
              value = presetOptionLabel(MAX_WIDTH_OPTIONS, layout.maxWidth, "${layout.maxWidth}px"),
              onClick = { openEditor(PresetEditorField.MaxWidth) },
            )
          }

          is PresetLayout.Paginated -> {
            CardDivider()
            PresetSettingsRow(
              label = "페이지 크기",
              value = pageLayoutSummaryLabel(layout),
              onClick = { openEditor(PresetEditorField.PageSize) },
            )
            CardDivider()
            PresetSettingsRow(
              label = "여백",
              value = pageMarginSummaryLabel(layout),
              onClick = { openEditor(PresetEditorField.PageMargin) },
            )
          }

          is PresetLayout.Unknown -> Unit
        }
      }

      PresetSettingsSection(title = "세부 레이아웃") {
        PresetSettingsRow(
          label = "첫 줄 들여쓰기",
          value = presetOptionLabel(PARAGRAPH_INDENT_OPTIONS, template.paragraphIndent),
          onClick = { openEditor(PresetEditorField.ParagraphIndent) },
        )
        CardDivider()
        PresetSettingsRow(
          label = "문단 간격",
          value = presetOptionLabel(BLOCK_GAP_OPTIONS, template.blockGap),
          onClick = { openEditor(PresetEditorField.BlockGap) },
        )
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun PresetTopBarMenu(
  model: PresetSettingsViewModel,
  actionScope: kotlinx.coroutines.CoroutineScope,
) {
  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.Ellipsis) },
    pane = { PresetTopBarMenuPane(model = model, actionScope = actionScope) },
  )
}

@Composable
context(_: PopoverScope)
private fun PresetTopBarMenuPane(
  model: PresetSettingsViewModel,
  actionScope: kotlinx.coroutines.CoroutineScope,
) {
  val dialog = LocalDialog.current
  val toast = LocalToast.current

  Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
    PopoverList(
      items =
        listOf(
          PopoverListItem(
            content = {
              Row(
                modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
              ) {
                Icon(
                  icon = Lucide.RotateCcw,
                  modifier = Modifier.size(18.dp),
                  tint = AppTheme.colors.danger,
                )
                Text(
                  text = "프리셋 초기화",
                  style = AppTheme.typography.action,
                  color = AppTheme.colors.danger,
                )
              }
            },
            onSelected = {
              close()
              if (model.query.state is QueryState.Success) {
                actionScope.launch {
                  val result =
                    dialog.confirm(
                      title = "프리셋 초기화",
                      message = "모든 프리셋 설정을 기본값으로 되돌려요. 이 작업은 되돌릴 수 없어요.",
                      confirmText = "초기화",
                      confirmIsDestructive = true,
                    )
                  if (result is DialogResult.Resolved) {
                    model.resetTemplate().withDefaultExceptionHandler(toast)
                  }
                }
              }
            },
          )
        )
    )
  }
}

@Composable
private fun PresetSettingsSection(title: String, content: @Composable () -> Unit) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(text = title, modifier = Modifier.padding(top = 4.dp))

    CardSurface(modifier = Modifier.fillMaxWidth()) { Column(content = { content() }) }
  }
}

@Composable
private fun PresetSettingsRow(label: String, value: String, onClick: suspend () -> Unit) {
  CardRow(onClick = onClick) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Text(text = label, style = AppTheme.typography.label, modifier = Modifier.weight(1f))

      Text(
        text = value,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )

      Icon(
        icon = Lucide.ChevronRight,
        modifier = Modifier.size(16.dp),
        tint = AppTheme.colors.textTertiary,
      )
    }
  }
}

private fun fontFamilySummaryLabel(
  template: PresetTemplate,
  fontFamilyOptions: List<PresetOption<String>>,
): String {
  return fontFamilyOptions.firstOrNull { it.value == template.fontFamily }?.label
    ?: template.fontFamily
}

private fun fontWeightSummaryLabel(
  template: PresetTemplate,
  fontWeightOptions: List<PresetOption<Int>>,
): String {
  val options = if (fontWeightOptions.isNotEmpty()) fontWeightOptions else FONT_WEIGHT_OPTIONS
  return presetOptionLabel(options, template.fontWeight, template.fontWeight.toString())
}

@Composable
context(_: SheetScope<Unit>)
private fun FontFamilyContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val toast = LocalToast.current
  var isSaving by remember { mutableStateOf(false) }
  var selectedFamilyName by remember { mutableStateOf(template.fontFamily) }

  val families =
    remember(model.activeDocumentFontFamilies) {
      model.activeDocumentFontFamilies.sortedBy { it.displayName.lowercase() }
    }

  PresetInstantSheetLayout(title = "폰트 패밀리") {
    if (families.isEmpty()) {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Text(
          text = "선택할 수 있는 폰트가 없어요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.padding(16.dp),
        )
      }
    } else {
      SheetOptionList(items = families) { family ->
        val specimen = representativeFontEntry(family, template.fontWeight)
        SheetOptionRow(
          selected = selectedFamilyName == family.familyName,
          enabled = !isSaving,
          onClick = {
            if (isSaving) return@SheetOptionRow
            selectedFamilyName = family.familyName
            isSaving = true
          },
        ) {
          FontSpecimen(
            text = family.displayName,
            fontId = specimen?.id,
            weight = specimen?.weight,
            style = AppTheme.typography.body,
            fallbackTexts =
              familySpecimenFallbacks(
                displayName = family.displayName,
                familyName = family.familyName,
              ),
          )
        }
      }
    }
  }

  val selectedFamily = families.firstOrNull { it.familyName == selectedFamilyName }
  if (isSaving && selectedFamily != null) {
    rememberImmediateSave(
      key = selectedFamilyName,
      toast = toast,
      onFinish = { success ->
        isSaving = false
        if (success) {
          complete(Unit)
        }
      },
    ) {
      model.saveTemplate(
        template
          .withFontFamily(selectedFamily.familyName)
          .withFontWeight(
            closestWeight(template.fontWeight, selectedFamily.fonts.map { it.weight })
          )
      )
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun FontWeightContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val toast = LocalToast.current
  val family = model.activeDocumentFontFamilies.firstOrNull { it.familyName == template.fontFamily }
  val fonts = family?.fonts.orEmpty().distinctBy { it.weight }.sortedBy { it.weight }
  var isSaving by remember { mutableStateOf(false) }
  var selectedWeight by remember {
    mutableStateOf(
      if (fonts.isEmpty()) {
        template.fontWeight
      } else {
        closestWeight(template.fontWeight, fonts.map { it.weight })
      }
    )
  }

  PresetInstantSheetLayout(title = "폰트 굵기") {
    if (fonts.isEmpty()) {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Text(
          text = "선택할 수 있는 폰트 굵기가 없어요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.padding(16.dp),
        )
      }
    } else {
      SheetOptionList(items = fonts) { font ->
        val fontLabel = fontWeightLabel(font.weight, font.subfamilyDisplayName)

        SheetOptionRow(
          selected = selectedWeight == font.weight,
          enabled = !isSaving,
          onClick = {
            if (isSaving) return@SheetOptionRow
            selectedWeight = font.weight
            isSaving = true
          },
        ) {
          FontSpecimen(
            text = fontLabel,
            fontId = font.id,
            weight = font.weight,
            style = AppTheme.typography.body,
            fallbackTexts =
              weightSpecimenFallbacks(
                label = fontLabel,
                subfamilyDisplayName = font.subfamilyDisplayName,
                weight = font.weight,
              ),
          )
        }
      }
    }
  }

  if (isSaving) {
    rememberImmediateSave(
      key = selectedWeight,
      toast = toast,
      onFinish = { success ->
        isSaving = false
        if (success) {
          complete(Unit)
        }
      },
    ) {
      model.saveTemplate(template.withFontWeight(selectedWeight))
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun FontSizeContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val form = remember(scope, template.fontSize) { FontSizeSheetForm(scope, template.fontSize) }
  var isSaving by remember { mutableStateOf(false) }
  var errorText by remember { mutableStateOf<String?>(null) }
  val draftValue = parsePointInput(form.fontSize.value)?.coerceIn(MIN_FONT_SIZE, MAX_FONT_SIZE)

  LaunchedEffect(form.fontSize.value) { errorText = null }

  PresetSheetLayout(
    title = "폰트 크기",
    isSaving = isSaving,
    onSave = {
      if (draftValue == null) {
        errorText = "폰트 크기를 올바르게 입력해 주세요."
        return@PresetSheetLayout
      }

      isSaving = true
      model
        .saveTemplate(template.withFontSize(draftValue))
        .withDefaultExceptionHandler(toast)
        .onOk { complete(Unit) }
      isSaving = false
    },
  ) {
    TextField(
      field = form.fontSize,
      label = "폰트 크기 (pt)",
      labelPosition = LabelPosition.Internal,
      placeholder = "12",
      keyboardType = KeyboardType.Number,
      modifier = Modifier.fillMaxWidth(),
    )

    Text(
      text = "빠른 선택",
      modifier = Modifier.padding(horizontal = 16.dp),
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )

    FlowRow(
      modifier = Modifier.padding(horizontal = 16.dp),
      horizontalArrangement = Arrangement.spacedBy(12.dp),
      verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
      FONT_SIZE_OPTIONS.forEach { option ->
        PresetQuickSelectButton(
          label = option.label,
          selected = draftValue == option.value,
          onClick = {
            if (isSaving) return@PresetQuickSelectButton
            form.fontSize.setValue(formatPresetPointValue(option.value))
          },
        )
      }
    }

    errorText?.let { message ->
      Text(text = message, style = AppTheme.typography.caption, color = AppTheme.colors.danger)
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun LayoutModeContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val initialMode =
    when (template.layout) {
      is PresetLayout.Paginated -> "paginated"
      else -> "continuous"
    }

  PresetOptionContent(
    title = "레이아웃 모드",
    initialValue = initialMode,
    options =
      listOf(
        PresetOption(label = "스크롤", value = "continuous"),
        PresetOption(label = "페이지", value = "paginated"),
      ),
    onSaveValue = { nextMode ->
      val nextLayout =
        when (nextMode) {
          "paginated" -> template.layout as? PresetLayout.Paginated ?: createPaginatedLayout("a4")
          else -> template.layout as? PresetLayout.Continuous ?: PresetLayout.Continuous()
        }
      model.saveTemplate(template.withLayout(nextLayout))
    },
  )
}

@Composable
context(_: SheetScope<Unit>)
private fun MaxWidthContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val layout = template.layout as? PresetLayout.Continuous ?: PresetLayout.Continuous()

  PresetOptionContent(
    title = "본문 폭",
    initialValue = layout.maxWidth,
    options = MAX_WIDTH_OPTIONS,
    onSaveValue = { next -> model.saveTemplate(template.withLayout(layout.withMaxWidth(next))) },
  )
}

@Composable
context(_: SheetScope<Unit>)
private fun PageSizeContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val toast = LocalToast.current
  val initialLayout = template.layout as? PresetLayout.Paginated ?: createPaginatedLayout("a4")
  val scope = rememberCoroutineScope()
  val form = remember(scope, initialLayout) { PageSizeSheetForm(scope, initialLayout) }
  var isSaving by remember { mutableStateOf(false) }
  var draftLayout by remember { mutableStateOf(initialLayout) }
  var errorText by remember { mutableStateOf<String?>(null) }

  LaunchedEffect(form.widthMm.value, form.heightMm.value) {
    errorText = null

    val parsedWidth = parseMillimeterInput(form.widthMm.value, min = 100) ?: return@LaunchedEffect
    val parsedHeight = parseMillimeterInput(form.heightMm.value, min = 100) ?: return@LaunchedEffect

    draftLayout =
      clampPaginatedLayout(
        draftLayout.withPageWidth(mmToPx(parsedWidth)).withPageHeight(mmToPx(parsedHeight))
      )
  }

  PresetSheetLayout(
    title = "페이지 크기",
    isSaving = isSaving,
    onSave = {
      val parsedWidth = parseMillimeterInput(form.widthMm.value, min = 100)
      val parsedHeight = parseMillimeterInput(form.heightMm.value, min = 100)

      if (parsedWidth == null || parsedHeight == null) {
        errorText = "페이지 크기를 올바르게 입력해 주세요."
        return@PresetSheetLayout
      }

      val nextLayout =
        clampPaginatedLayout(
          draftLayout.withPageWidth(mmToPx(parsedWidth)).withPageHeight(mmToPx(parsedHeight))
        )

      isSaving = true
      model.saveTemplate(template.withLayout(nextLayout)).withDefaultExceptionHandler(toast).onOk {
        complete(Unit)
      }
      isSaving = false
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
      Text(
        text = "가로 / 세로 (mm)",
        modifier = Modifier.padding(horizontal = 16.dp),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )

      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        TextField(
          field = form.widthMm,
          label = "가로",
          labelPosition = LabelPosition.Internal,
          placeholder = "210",
          keyboardType = KeyboardType.Number,
          modifier = Modifier.weight(1f),
        )

        Text(text = "×", style = AppTheme.typography.title, color = AppTheme.colors.textTertiary)

        TextField(
          field = form.heightMm,
          label = "세로",
          labelPosition = LabelPosition.Internal,
          placeholder = "297",
          keyboardType = KeyboardType.Number,
          modifier = Modifier.weight(1f),
        )
      }

      Text(
        text = "빠른 선택",
        modifier = Modifier.padding(horizontal = 16.dp),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )

      Row(
        modifier = Modifier.padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        PAGE_LAYOUT_OPTIONS.forEach { option ->
          PresetQuickSelectButton(
            label = option.label.substringBefore(" "),
            selected = pageLayoutPresetOrCustom(draftLayout) == option.value,
            onClick = {
              draftLayout = createPaginatedLayout(option.value)
              form.widthMm.setValue(pxToMm(draftLayout.pageWidth).toString())
              form.heightMm.setValue(pxToMm(draftLayout.pageHeight).toString())
            },
          )
        }
      }

      errorText?.let { message ->
        Text(text = message, style = AppTheme.typography.caption, color = AppTheme.colors.danger)
      }
    }
  }
}

@Composable
private fun PresetQuickSelectButton(label: String, selected: Boolean, onClick: suspend () -> Unit) {
  InteractionScope {
    Box(
      modifier = Modifier.clickable(onClick).pressScale().padding(vertical = 2.dp),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = label,
        style =
          if (selected) {
            AppTheme.typography.action.copy(fontWeight = FontWeight.W700)
          } else {
            AppTheme.typography.action
          },
        color = AppTheme.colors.brand,
      )
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun PageMarginContent(model: PresetSettingsViewModel, template: PresetTemplate) {
  val toast = LocalToast.current
  val layout = template.layout as? PresetLayout.Paginated ?: createPaginatedLayout("a4")
  val scope = rememberCoroutineScope()
  val form = remember(scope, layout) { PageMarginSheetForm(scope, layout) }
  var isSaving by remember { mutableStateOf(false) }
  var errorText by remember { mutableStateOf<String?>(null) }

  LaunchedEffect(form.topMm.value, form.bottomMm.value, form.leftMm.value, form.rightMm.value) {
    errorText = null
  }

  PresetSheetLayout(
    title = "여백",
    isSaving = isSaving,
    onSave = {
      val parsedTop = parseMillimeterInput(form.topMm.value, min = 0)
      val parsedBottom = parseMillimeterInput(form.bottomMm.value, min = 0)
      val parsedLeft = parseMillimeterInput(form.leftMm.value, min = 0)
      val parsedRight = parseMillimeterInput(form.rightMm.value, min = 0)

      if (parsedTop == null || parsedBottom == null || parsedLeft == null || parsedRight == null) {
        errorText = "여백 값을 올바르게 입력해 주세요."
        return@PresetSheetLayout
      }

      var nextLayout = layout
      nextLayout = nextLayout.withPageMarginTop(mmToPx(parsedTop))
      nextLayout = nextLayout.withPageMarginBottom(mmToPx(parsedBottom))
      nextLayout = nextLayout.withPageMarginLeft(mmToPx(parsedLeft))
      nextLayout = nextLayout.withPageMarginRight(mmToPx(parsedRight))
      nextLayout = clampPaginatedLayout(nextLayout)

      isSaving = true
      model.saveTemplate(template.withLayout(nextLayout)).withDefaultExceptionHandler(toast).onOk {
        complete(Unit)
      }
      isSaving = false
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth()) {
      Text(
        text = "${pxToMm(layout.pageWidth)} × ${pxToMm(layout.pageHeight)}mm 페이지",
        modifier = Modifier.padding(horizontal = 16.dp),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )

      Spacer(modifier = Modifier.height(12.dp))

      TextField(
        field = form.topMm,
        label = "위쪽 여백 (mm)",
        labelPosition = LabelPosition.Internal,
        placeholder = "25",
        keyboardType = KeyboardType.Number,
      )

      TextField(
        field = form.bottomMm,
        label = "아래쪽 여백 (mm)",
        labelPosition = LabelPosition.Internal,
        placeholder = "25",
        keyboardType = KeyboardType.Number,
      )

      TextField(
        field = form.leftMm,
        label = "왼쪽 여백 (mm)",
        labelPosition = LabelPosition.Internal,
        placeholder = "25",
        keyboardType = KeyboardType.Number,
      )

      TextField(
        field = form.rightMm,
        label = "오른쪽 여백 (mm)",
        labelPosition = LabelPosition.Internal,
        placeholder = "25",
        keyboardType = KeyboardType.Number,
      )
      errorText?.let { message ->
        Text(text = message, style = AppTheme.typography.caption, color = AppTheme.colors.danger)
      }
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun <T> PresetOptionContent(
  title: String,
  initialValue: T,
  options: List<PresetOption<T>>,
  onSaveValue: suspend (T) -> Result<Unit, Nothing>,
) {
  val toast = LocalToast.current
  var isSaving by remember { mutableStateOf(false) }
  var selectedValue by remember { mutableStateOf(initialValue) }

  PresetInstantSheetLayout(title = title) {
    SheetOptionList(items = options) { option ->
      SheetOptionRow(
        selected = selectedValue == option.value,
        enabled = !isSaving,
        onClick = {
          if (isSaving) return@SheetOptionRow
          selectedValue = option.value
          isSaving = true
        },
      ) {
        Text(option.label, style = AppTheme.typography.body)
      }
    }
  }

  if (isSaving) {
    rememberImmediateSave(
      key = selectedValue,
      toast = toast,
      onFinish = { success ->
        isSaving = false
        if (success) {
          complete(Unit)
        }
      },
    ) {
      onSaveValue(selectedValue)
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun PresetColorContent(
  title: String,
  initialValue: String,
  options: List<PresetOption<String>>,
  background: Boolean,
  onSaveValue: suspend (String) -> Result<Unit, Nothing>,
) {
  val toast = LocalToast.current
  var isSaving by remember { mutableStateOf(false) }
  var selectedValue by remember { mutableStateOf(initialValue) }

  PresetInstantSheetLayout(title = title) {
    SheetOptionList(items = options) { option ->
      SheetOptionRow(
        selected = selectedValue == option.value,
        enabled = !isSaving,
        onClick = {
          if (isSaving) return@SheetOptionRow
          selectedValue = option.value
          isSaving = true
        },
      ) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          PresetColorSwatch(
            color = presetColor(option.value, background),
            isNone = background && option.value == "none",
            shape = if (background) PresetColorSwatchShape.Square else PresetColorSwatchShape.Circle,
          )
          Text(option.label, style = AppTheme.typography.body)
        }
      }
    }
  }

  if (isSaving) {
    rememberImmediateSave(
      key = selectedValue,
      toast = toast,
      onFinish = { success ->
        isSaving = false
        if (success) {
          complete(Unit)
        }
      },
    ) {
      onSaveValue(selectedValue)
    }
  }
}

@Composable
context(_: SheetScope<R>)
private fun <R> PresetSheetLayout(
  title: String,
  isSaving: Boolean,
  saveEnabled: Boolean = true,
  onSave: suspend () -> Unit,
  content: @Composable ColumnScope.() -> Unit,
) {
  SheetLayout(
    padding = PresetSheetPadding,
    verticalSpacing = 8.dp,
    header = {
      SheetBar(
        leading = {
          SheetBarTextButton(
            text = "취소",
            color = AppTheme.colors.brand,
            enabled = !isSaving,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = title,
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        },
        trailing = {
          SheetBarTextButton(
            text = "저장",
            color = AppTheme.colors.brand,
            enabled = saveEnabled,
            loading = isSaving,
            onClick = onSave,
          )
        },
      )
    },
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp), content = content)
  }
}

@Composable
context(_: SheetScope<R>)
private fun <R> PresetInstantSheetLayout(
  title: String,
  content: @Composable ColumnScope.() -> Unit,
) {
  SheetLayout(
    padding = PresetSheetPadding,
    verticalSpacing = 8.dp,
    header = {
      SheetBar(
        center = {
          Text(
            text = title,
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp), content = content)
  }
}

private enum class PresetColorSwatchShape {
  Circle,
  Square,
}

@Composable
private fun PresetColorSwatch(color: Color?, isNone: Boolean, shape: PresetColorSwatchShape) {
  val resolvedShape =
    when (shape) {
      PresetColorSwatchShape.Circle -> CircleShape
      PresetColorSwatchShape.Square -> RoundedCornerShape(4.dp)
    }
  val slashColor = AppTheme.colors.textMuted

  Box(
    modifier =
      Modifier.size(16.dp)
        .border(1.dp, AppTheme.colors.borderDefault, resolvedShape)
        .background(color ?: Color.Transparent, resolvedShape),
    contentAlignment = Alignment.Center,
  ) {
    if (isNone) {
      Canvas(modifier = Modifier.size(12.dp)) {
        drawLine(
          color = slashColor,
          start = Offset(2f, size.height - 2f),
          end = Offset(size.width - 2f, 2f),
          strokeWidth = 1.dp.toPx(),
        )
      }
    }
  }
}

@Composable
private fun rememberImmediateSave(
  key: Any?,
  toast: Toast,
  onFinish: (Boolean) -> Unit,
  action: suspend () -> Result<Unit, Nothing>,
) {
  androidx.compose.runtime.LaunchedEffect(key) {
    onFinish(action().withDefaultExceptionHandler(toast).isOk)
  }
}

private fun representativeFontEntry(family: PresetFontFamily, targetWeight: Int): PresetFontEntry? {
  val closest = closestWeight(targetWeight, family.fonts.map { it.weight })
  return family.fonts.firstOrNull { it.weight == closest } ?: family.fonts.firstOrNull()
}

private fun fontWeightLabel(weight: Int, subfamilyDisplayName: String?): String {
  return FONT_WEIGHT_OPTIONS.firstOrNull { it.value == weight }?.label
    ?: subfamilyDisplayName?.takeIf { it.isNotBlank() }?.let { "$it ($weight)" }
    ?: weight.toString()
}

private fun parsePointInput(value: String): Int? {
  val numeric = value.trim().replace(',', '.').toDoubleOrNull() ?: return null
  if (numeric <= 0) return null
  return (numeric * 100).roundToInt()
}

private fun parseMillimeterInput(value: String, min: Int): Int? {
  val numeric = value.trim().replace(',', '.').toDoubleOrNull() ?: return null
  if (numeric < min) return null
  return numeric.roundToInt()
}

private fun clampPaginatedLayout(layout: PresetLayout.Paginated): PresetLayout.Paginated {
  var draft = layout
  draft =
    draft.withPageMarginTop(
      draft.pageMarginTop.coerceIn(0, getMaxMargin(PageMarginSide.Top, draft))
    )
  draft =
    draft.withPageMarginBottom(
      draft.pageMarginBottom.coerceIn(0, getMaxMargin(PageMarginSide.Bottom, draft))
    )
  draft =
    draft.withPageMarginLeft(
      draft.pageMarginLeft.coerceIn(0, getMaxMargin(PageMarginSide.Left, draft))
    )
  draft =
    draft.withPageMarginRight(
      draft.pageMarginRight.coerceIn(0, getMaxMargin(PageMarginSide.Right, draft))
    )
  return draft
}

private fun presetColor(value: String, background: Boolean): Color? {
  return when (value) {
    "none" -> if (background) null else Color.Transparent
    "black" -> Color(0xFF111827)
    "darkgray" -> Color(0xFF374151)
    "gray" -> if (background) Color(0xFFF3F4F6) else Color(0xFF6B7280)
    "lightgray" -> Color(0xFFD1D5DB)
    "white" -> Color(0xFFFFFFFF)
    "red" -> if (background) Color(0xFFFEE2E2) else AppColor.light.red.s500
    "orange" -> if (background) Color(0xFFFFEDD5) else Color(0xFFF97316)
    "amber" -> AppColor.light.amber.s500
    "yellow" -> if (background) Color(0xFFFEF9C3) else Color(0xFFEAB308)
    "lime" -> Color(0xFF84CC16)
    "green" -> if (background) Color(0xFFDCFCE7) else AppColor.light.green.s500
    "emerald" -> Color(0xFF10B981)
    "teal" -> Color(0xFF14B8A6)
    "cyan" -> Color(0xFF06B6D4)
    "sky" -> Color(0xFF0EA5E9)
    "blue" -> if (background) Color(0xFFDBEAFE) else AppColor.light.blue.s500
    "indigo" -> Color(0xFF6366F1)
    "violet" -> Color(0xFF8B5CF6)
    "purple" -> if (background) Color(0xFFF3E8FF) else Color(0xFFA855F7)
    "fuchsia" -> Color(0xFFD946EF)
    "pink" -> Color(0xFFEC4899)
    "rose" -> Color(0xFFF43F5E)
    else -> null
  }
}
