package co.typie.screen.settings.presetsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.zIndex
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.EditorOption
import co.typie.editor.EditorTheme
import co.typie.editor.EditorValues
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.matchWeight
import co.typie.ext.clickable
import co.typie.ext.excludeBottom
import co.typie.ext.imePadding
import co.typie.ext.verticalScroll
import co.typie.graphql.PresetSettingsScreen_Query
import co.typie.icons.Lucide
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.FontSpecimen
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

@Composable
fun PresetSettingsScreen() {
  val model = viewModel { PresetSettingsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val toast = LocalToast.current
  val sheet = LocalSheet.current

  suspend fun save(preset: Preset) {
    model.updatePreset(preset).withDefaultExceptionHandler(toast)
  }

  ProvideTopBar(
    center = { Text("프리셋", style = AppTheme.typography.title) },
    trailing = {
      PresetMenu(
        onReset = { scope.launch { model.resetPreset().withDefaultExceptionHandler(toast) } }
      )
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    loadable = model.query,
    background = AppTheme.colors.surfaceInset,
    contentPadding = PaddingValues.Zero,
  ) { contentPadding ->
    val colors = AppTheme.colors
    val previewHeight = 200.dp
    val previewShape = RoundedCornerShape(bottomStart = AppShapes.xl, bottomEnd = AppShapes.xl)

    Box(modifier = Modifier.fillMaxSize().imePadding().padding(contentPadding.excludeBottom())) {
      Column(
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(scrollState)
            .background(colors.surfaceDefault)
            .padding(top = previewHeight + 12.dp, bottom = contentPadding.calculateBottomPadding())
            .padding(AppTheme.spacings.scrollBottomPadding)
      ) {
        FontSection(model = model, sheet = sheet, onSave = ::save)

        SectionDivider()

        SpacingSection(model = model, onSave = ::save)

        SectionDivider()

        ColorSection(model = model, onSave = ::save)

        SectionDivider()

        LayoutSection(model = model, sheet = sheet, onSave = ::save)
      }

      Box(modifier = Modifier.fillMaxWidth()) {
        Box(
          modifier =
            Modifier.fillMaxWidth()
              .height(previewHeight)
              .background(colors.surfaceInset, previewShape)
              .zIndex(1f)
        )

        Box(
          modifier =
            Modifier.fillMaxWidth()
              .height(16.dp + AppShapes.xl / 2)
              .offset(y = previewHeight - AppShapes.xl / 2)
              .background(
                Brush.verticalGradient(
                  colors = listOf(colors.surfaceInset, colors.surfaceInset.copy(alpha = 0f))
                )
              )
        )
      }
    }
  }
}

@Composable
private fun PresetMenu(onReset: () -> Unit) {
  val scope = rememberCoroutineScope()

  val dialog = LocalDialog.current
  val colors = AppTheme.colors

  PopoverMenu(anchor = { TopBarButton(icon = Lucide.Ellipsis) }) {
    item(icon = Lucide.RotateCcw, label = "프리셋 초기화", color = colors.danger) {
      scope.launch {
        val result =
          dialog.confirm(
            title = "프리셋 초기화",
            message = "모든 프리셋 설정을 기본값으로 되돌려요. 이 작업은 되돌릴 수 없어요.",
            confirmText = "초기화",
            confirmIsDestructive = true,
          )
        if (result is DialogResult.Resolved) {
          onReset()
        }
      }
    }
  }
}

@Composable
private fun FontSection(
  model: PresetSettingsViewModel,
  sheet: Sheet,
  onSave: suspend (Preset) -> Unit,
) {
  val selectedFamily = model.fontFamilies.firstOrNull { it.familyName == model.preset.fontFamily }

  val weightOptions =
    selectedFamily
      ?.fonts
      ?.distinctBy { it.weight }
      ?.sortedBy { it.weight }
      ?.map { font ->
        EditorOption(
          label =
            EditorValues.fontWeight.firstOrNull { it.value == font.weight }?.label
              ?: font.subfamilyDisplayName?.let { "$it (${font.weight})" }
              ?: "${font.weight}",
          value = font.weight,
        )
      } ?: EditorValues.fontWeight

  PresetSection(title = "글꼴") {
    CardRow(
      onClick = {
        sheet.present {
          FontFamilySheet(model.fontFamilies, model.preset) { newFamily ->
            val availableWeights =
              model.fontFamilies
                .firstOrNull { it.familyName == newFamily }
                ?.fonts
                ?.map { it.weight }
                ?.sorted() ?: emptyList()
            val matchedWeight =
              matchWeight(availableWeights, model.preset.fontWeight) ?: model.preset.fontWeight
            model.updatePreset(
              model.preset.copy(fontFamily = newFamily, fontWeight = matchedWeight)
            )
          }
        }
      }
    ) {
      Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "패밀리", style = AppTheme.typography.label, color = AppTheme.colors.textMuted)

        if (selectedFamily != null) {
          val representativeFont = selectedFamily.representativeFont
          FontSpecimen(
            fontId = representativeFont.id,
            text = selectedFamily.displayName,
            fallbackTexts = listOf(selectedFamily.familyName),
            style = TextStyle(fontSize = 17.sp, fontWeight = FontWeight(representativeFont.weight)),
          )
        } else {
          Text(
            text = model.preset.fontFamily,
            style = AppTheme.typography.action,
            color = AppTheme.colors.textMuted,
          )
        }
      }

      Icon(
        icon = Lucide.ChevronRight,
        modifier = Modifier.size(16.dp),
        tint = AppTheme.colors.textMuted,
      )
    }

    CardDivider()

    PresetSettingRow(label = "굵기") {
      PresetChipRow(
        options = weightOptions,
        selected = model.preset.fontWeight,
        onSelect = { onSave(model.preset.copy(fontWeight = it)) },
      )
    }

    CardDivider()

    PresetSnapSlider(
      label = "크기",
      value = model.preset.fontSize,
      onValueChange = { onSave(model.preset.copy(fontSize = it)) },
      range = 800..2400,
      sliderStep = 100,
      inputStep = 50,
      formatValue = { "${formatPresetPointValue(it)}pt" },
      parseValue = { input -> input.trim().toDoubleOrNull()?.let { (it * 100).roundToInt() } },
      unitSuffix = "pt",
      fullRange = EditorValues.minFontSize..EditorValues.maxFontSize,
      modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
    )
  }
}

@Composable
private fun SpacingSection(model: PresetSettingsViewModel, onSave: suspend (Preset) -> Unit) {
  PresetSection(title = "간격") {
    PresetSnapSlider(
      label = "자간",
      value = model.preset.letterSpacing,
      onValueChange = { onSave(model.preset.copy(letterSpacing = it)) },
      range = -10..40,
      sliderStep = 5,
      inputStep = 1,
      formatValue = { "${it}%" },
      parseValue = { input -> input.trim().toDoubleOrNull()?.roundToInt() },
      unitSuffix = "%",
      modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
    )

    CardDivider()

    PresetSnapSlider(
      label = "행간",
      value = model.preset.lineHeight,
      onValueChange = { onSave(model.preset.copy(lineHeight = it)) },
      range = 80..220,
      sliderStep = 10,
      inputStep = 1,
      formatValue = { "${it}%" },
      parseValue = { input -> input.trim().toDoubleOrNull()?.roundToInt() },
      unitSuffix = "%",
      modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
    )

    CardDivider()

    PresetSettingRow(label = "들여쓰기") {
      PresetChipRow(
        options = EditorValues.paragraphIndent,
        selected = model.preset.paragraphIndent,
        onSelect = { onSave(model.preset.copy(paragraphIndent = it)) },
      )
    }

    CardDivider()

    PresetSettingRow(label = "문단 간격") {
      PresetChipRow(
        options = EditorValues.blockGap,
        selected = model.preset.blockGap,
        onSelect = { onSave(model.preset.copy(blockGap = it)) },
      )
    }
  }
}

@Composable
private fun ColorSection(model: PresetSettingsViewModel, onSave: suspend (Preset) -> Unit) {
  val variant = currentEditorThemeVariant()
  val editorTheme = remember(variant) { EditorTheme.resolve(variant) }

  PresetSection(title = "색상") {
    PresetSettingRow(label = "글자 색") {
      PresetSwatchRow(
        options = EditorValues.textColor,
        selected = model.preset.textColor,
        onSelect = { onSave(model.preset.copy(textColor = it)) },
        theme = editorTheme,
        cornerRadius = 50.dp,
      )
    }

    CardDivider()

    PresetSettingRow(label = "배경 색") {
      PresetSwatchRow(
        options = EditorValues.textBackgroundColor,
        selected = model.preset.backgroundColor,
        onSelect = { onSave(model.preset.copy(backgroundColor = it)) },
        theme = editorTheme,
        cornerRadius = AppShapes.sm * 2,
      )
    }
  }
}

@Composable
private fun LayoutSection(
  model: PresetSettingsViewModel,
  sheet: Sheet,
  onSave: suspend (Preset) -> Unit,
  scope: CoroutineScope = rememberCoroutineScope(),
) {
  val layoutModeOptions =
    listOf(
      EditorOption(label = "스크롤", value = "continuous"),
      EditorOption(label = "페이지", value = "paginated"),
    )

  val currentLayoutMode =
    when (model.preset.layout) {
      is PresetPageLayout.Paginated -> "paginated"
      is PresetPageLayout.Continuous -> "continuous"
    }

  PresetSection(title = "레이아웃") {
    PresetSettingRow(label = "모드") {
      PresetChipRow(
        options = layoutModeOptions,
        selected = currentLayoutMode,
        onSelect = { mode ->
          when (mode) {
            "paginated" -> onSave(model.preset.copy(layout = PresetPageLayout.Paginated()))
            "continuous" -> onSave(model.preset.copy(layout = PresetPageLayout.Continuous()))
          }
        },
      )
    }

    when (val layout = model.preset.layout) {
      is PresetPageLayout.Paginated -> {
        val pageOption =
          EditorValues.pageLayout.firstOrNull {
            it.layout.pageWidth == layout.pageWidth && it.layout.pageHeight == layout.pageHeight
          }
        val currentPageSize = pageOption?.value ?: "custom"

        val pageSizeOptions =
          EditorValues.pageLayout.map {
            EditorOption(label = it.label.substringBefore(" "), value = it.value)
          }

        val margins = pageOption?.margins ?: emptyList()
        val selectedMargin =
          margins
            .firstOrNull { m ->
              layout.pageMarginTop == m.top &&
                layout.pageMarginBottom == m.bottom &&
                layout.pageMarginLeft == m.left &&
                layout.pageMarginRight == m.right
            }
            ?.value ?: "custom"

        CardDivider()

        PresetSettingRow(label = "페이지 크기") {
          PresetChipRow(
            options = pageSizeOptions,
            selected = currentPageSize,
            onSelect = { value ->
              val option =
                EditorValues.pageLayout.firstOrNull { it.value == value } ?: return@PresetChipRow
              val currentMarginName =
                margins
                  .firstOrNull { m ->
                    layout.pageMarginTop == m.top &&
                      layout.pageMarginBottom == m.bottom &&
                      layout.pageMarginLeft == m.left &&
                      layout.pageMarginRight == m.right
                  }
                  ?.value
              val newMargin =
                if (currentMarginName != null) {
                  option.margins.firstOrNull { it.value == currentMarginName }
                    ?: option.margins.first { it.value == "normal" }
                } else {
                  null
                }
              onSave(
                model.preset.copy(
                  layout =
                    PresetPageLayout.Paginated(
                      pageWidth = option.layout.pageWidth,
                      pageHeight = option.layout.pageHeight,
                      pageMarginTop = newMargin?.top ?: layout.pageMarginTop,
                      pageMarginBottom = newMargin?.bottom ?: layout.pageMarginBottom,
                      pageMarginLeft = newMargin?.left ?: layout.pageMarginLeft,
                      pageMarginRight = newMargin?.right ?: layout.pageMarginRight,
                    )
                )
              )
            },
            trailing = {
              PresetTrailingChip(label = "사용자 정의", selected = currentPageSize == "custom") {
                sheet.present {
                  PresetPageLayoutSheet(preset = model.preset) { newLayout ->
                    scope.launch { model.updatePreset(model.preset.copy(layout = newLayout)) }
                  }
                }
              }
            },
          )
        }

        CardDivider()

        val marginOptions = margins.map { EditorOption(label = it.label, value = it.value) }

        PresetSettingRow(label = "여백") {
          PresetChipRow(
            options = marginOptions,
            selected = selectedMargin,
            onSelect = { value ->
              val margin = margins.firstOrNull { it.value == value } ?: return@PresetChipRow
              onSave(
                model.preset.copy(
                  layout =
                    PresetPageLayout.Paginated(
                      pageWidth = layout.pageWidth,
                      pageHeight = layout.pageHeight,
                      pageMarginTop = margin.top,
                      pageMarginBottom = margin.bottom,
                      pageMarginLeft = margin.left,
                      pageMarginRight = margin.right,
                    )
                )
              )
            },
            trailing = {
              PresetTrailingChip(label = "사용자 정의", selected = selectedMargin == "custom") {
                sheet.present {
                  PresetPageLayoutSheet(preset = model.preset) { newLayout ->
                    scope.launch { model.updatePreset(model.preset.copy(layout = newLayout)) }
                  }
                }
              }
            },
          )
        }
      }

      is PresetPageLayout.Continuous -> {
        CardDivider()

        PresetSettingRow(label = "본문 폭") {
          PresetChipRow(
            options = EditorValues.maxWidth,
            selected = layout.maxWidth,
            onSelect = {
              onSave(model.preset.copy(layout = PresetPageLayout.Continuous(maxWidth = it)))
            },
          )
        }
      }
    }
  }
}

@Composable
private fun SectionDivider() {
  Spacer(modifier = Modifier.fillMaxWidth().height(12.dp).background(AppTheme.colors.surfaceInset))
}

@Composable
private fun PresetSection(title: String, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = Modifier.fillMaxWidth().background(AppTheme.colors.surfaceDefault)) {
    Text(
      text = title,
      style = AppTheme.typography.title,
      modifier = Modifier.padding(start = 16.dp, top = 20.dp, bottom = 8.dp),
    )
    content()
  }
}

@Composable
private fun PresetSettingRow(label: String, content: @Composable () -> Unit) {
  Column(modifier = Modifier.fillMaxWidth().padding(vertical = 14.dp)) {
    Text(
      label,
      style = AppTheme.typography.label,
      color = AppTheme.colors.textMuted,
      modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
    )
    content()
  }
}

@Composable
private fun PresetTrailingChip(
  label: String,
  selected: Boolean = false,
  onClick: suspend () -> Unit,
) {
  val backgroundColor = if (selected) AppTheme.colors.textDefault else AppTheme.colors.surfaceInset
  val textColor = if (selected) AppTheme.colors.surfaceDefault else AppTheme.colors.textDefault
  Box(
    modifier =
      Modifier.clickable(onClick)
        .background(backgroundColor, AppShapes.circle)
        .padding(horizontal = 16.dp, vertical = 8.dp)
  ) {
    Text(text = label, style = AppTheme.typography.action, color = textColor)
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun FontFamilySheet(
  families: List<PresetSettingsScreen_Query.DocumentFontFamily>,
  preset: Preset,
  onSelect: suspend (String) -> Unit,
) {
  InstantSheetLayout(title = "폰트 패밀리") {
    SheetOptionList(items = families) { family ->
      val representativeFont = family.representativeFont
      SheetOptionRow(
        selected = preset.fontFamily == family.familyName,
        onClick = {
          onSelect(family.familyName)
          dismiss()
        },
      ) {
        FontSpecimen(
          fontId = representativeFont.id,
          text = family.displayName,
          fallbackTexts = listOf(family.familyName),
          style = TextStyle(fontSize = 15.sp, fontWeight = FontWeight(representativeFont.weight)),
        )
      }
    }
  }
}

@Composable
context(_: SheetScope<R>)
private fun <R> InstantSheetLayout(title: String, content: @Composable ColumnScope.() -> Unit) {
  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = title,
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp), content = content)
  }
}

private fun formatPresetPointValue(value: Int): String {
  val whole = value / 100
  val fraction = kotlin.math.abs(value % 100)
  if (fraction == 0) return whole.toString()
  return "$whole.${fraction.toString().padStart(2, '0')}".trimEnd('0').trimEnd('.')
}

private val PresetSettingsScreen_Query.DocumentFontFamily.representativeFont
  get() =
    this.fonts.reduce { previous, current ->
      val previousDiff = abs(previous.weight - 400)
      val currentDiff = abs(current.weight - 400)

      when {
        currentDiff < previousDiff -> current
        currentDiff == previousDiff && current.weight > previous.weight -> current
        else -> previous
      }
    }
