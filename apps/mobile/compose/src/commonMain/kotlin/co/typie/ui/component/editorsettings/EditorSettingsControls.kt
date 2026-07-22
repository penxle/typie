package co.typie.ui.component.editorsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.editor.DefaultRootPaginatedLayout
import co.typie.editor.EditorOption
import co.typie.editor.EditorValues
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.matchWeight
import co.typie.ext.clickable
import co.typie.graphql.fragment.EditorSettingsFontFamily_family
import co.typie.graphql.type.FontFamilySource
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.FontSpecimen
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

internal data class EditorStyleSettings(
  val fontFamily: String = "Pretendard",
  val fontSize: Int = 1200,
  val fontWeight: Int = 400,
  val letterSpacing: Int = 0,
  val lineHeight: Int = 160,
  val paragraphIndent: Int = 100,
  val blockGap: Int = 100,
)

internal fun List<EditorModifier>?.toEditorStyleSettings(): EditorStyleSettings =
  EditorStyleSettings(
    fontFamily = firstModifier<EditorModifier.FontFamily>()?.value ?: "Pretendard",
    fontSize = firstModifier<EditorModifier.FontSize>()?.value ?: 1200,
    fontWeight = firstModifier<EditorModifier.FontWeight>()?.value ?: 400,
    letterSpacing = firstModifier<EditorModifier.LetterSpacing>()?.value ?: 0,
    lineHeight = firstModifier<EditorModifier.LineHeight>()?.value ?: 160,
    paragraphIndent = firstModifier<EditorModifier.ParagraphIndent>()?.value ?: 100,
    blockGap = firstModifier<EditorModifier.BlockGap>()?.value ?: 100,
  )

private inline fun <reified T : EditorModifier> List<EditorModifier>?.firstModifier(): T? =
  this?.firstOrNull { it is T } as? T

internal fun EditorStyleSettings.toEditorModifiers(): List<EditorModifier> =
  listOf(
    EditorModifier.FontFamily(fontFamily),
    EditorModifier.FontSize(fontSize),
    EditorModifier.FontWeight(fontWeight),
    EditorModifier.LetterSpacing(letterSpacing),
    EditorModifier.LineHeight(lineHeight),
    EditorModifier.ParagraphIndent(paragraphIndent),
    EditorModifier.BlockGap(blockGap),
  )

internal fun EditorStyleSettings.changedEditorModifiersFrom(
  previous: EditorStyleSettings
): List<EditorModifier> = buildList {
  if (fontFamily != previous.fontFamily) add(EditorModifier.FontFamily(fontFamily))
  if (fontSize != previous.fontSize) add(EditorModifier.FontSize(fontSize))
  if (fontWeight != previous.fontWeight) add(EditorModifier.FontWeight(fontWeight))
  if (letterSpacing != previous.letterSpacing) add(EditorModifier.LetterSpacing(letterSpacing))
  if (lineHeight != previous.lineHeight) add(EditorModifier.LineHeight(lineHeight))
  if (paragraphIndent != previous.paragraphIndent) {
    add(EditorModifier.ParagraphIndent(paragraphIndent))
  }
  if (blockGap != previous.blockGap) add(EditorModifier.BlockGap(blockGap))
}

@Composable
internal fun EditorSettingsFontSection(
  style: EditorStyleSettings,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  sheet: Sheet,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSection(title = "글꼴") {
    EditorFontFamilyRow(
      style = style,
      fontFamilies = fontFamilies,
      sheet = sheet,
      onStyleChange = onStyleChange,
    )

    CardDivider()

    EditorFontWeightRow(style = style, fontFamilies = fontFamilies, onStyleChange = onStyleChange)

    CardDivider()

    EditorFontSizeSlider(style = style, onStyleChange = onStyleChange)
  }
}

@Composable
internal fun EditorSettingsBasicStyleSection(
  style: EditorStyleSettings,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  sheet: Sheet,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSection(title = "기본 서식") {
    EditorFontFamilyRow(
      style = style,
      fontFamilies = fontFamilies,
      sheet = sheet,
      onStyleChange = onStyleChange,
    )

    CardDivider()
    EditorFontWeightRow(style = style, fontFamilies = fontFamilies, onStyleChange = onStyleChange)
    CardDivider()
    EditorFontSizeSlider(style = style, onStyleChange = onStyleChange)
    CardDivider()
    EditorLetterSpacingSlider(style = style, onStyleChange = onStyleChange)
    CardDivider()
    EditorLineHeightSlider(style = style, onStyleChange = onStyleChange)
  }
}

@Composable
internal fun EditorSettingsSpacingSection(
  style: EditorStyleSettings,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSection(title = "간격") {
    EditorLetterSpacingSlider(style = style, onStyleChange = onStyleChange)
    CardDivider()
    EditorLineHeightSlider(style = style, onStyleChange = onStyleChange)
    CardDivider()
    EditorParagraphIndentRow(style = style, label = "들여쓰기", onStyleChange = onStyleChange)
    CardDivider()
    EditorBlockGapRow(style = style, label = "문단 간격", onStyleChange = onStyleChange)
  }
}

@Composable
internal fun EditorSettingsDetailLayoutSection(
  style: EditorStyleSettings,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSection(title = "세부 레이아웃") {
    EditorParagraphIndentRow(style = style, label = "첫 줄 들여쓰기", onStyleChange = onStyleChange)
    CardDivider()
    EditorBlockGapRow(style = style, label = "문단 사이 간격", onStyleChange = onStyleChange)
  }
}

@Composable
internal fun EditorSettingsLayoutSection(
  layout: LayoutMode,
  sheet: Sheet,
  onLayoutChange: suspend (LayoutMode) -> Unit,
) {
  val scope = rememberCoroutineScope()
  val layoutModeOptions =
    listOf(
      EditorOption(label = "스크롤", value = "continuous"),
      EditorOption(label = "페이지", value = "paginated"),
    )
  val currentLayoutMode =
    when (layout) {
      is LayoutMode.Paginated -> "paginated"
      is LayoutMode.Continuous -> "continuous"
    }

  EditorSettingsSection(title = "레이아웃") {
    EditorSettingsRow(label = "모드") {
      EditorSettingsChipRow(
        options = layoutModeOptions,
        selected = currentLayoutMode,
        onSelect = { mode ->
          when (mode) {
            "paginated" -> onLayoutChange(DefaultRootPaginatedLayout)
            "continuous" -> onLayoutChange(LayoutMode.Continuous(maxWidth = 600))
          }
        },
      )
    }

    when (layout) {
      is LayoutMode.Paginated -> {
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
            .firstOrNull { margin ->
              layout.pageMarginTop == margin.top &&
                layout.pageMarginBottom == margin.bottom &&
                layout.pageMarginLeft == margin.left &&
                layout.pageMarginRight == margin.right
            }
            ?.value ?: "custom"

        CardDivider()

        EditorSettingsRow(label = "페이지 크기") {
          EditorSettingsChipRow(
            options = pageSizeOptions,
            selected = currentPageSize,
            onSelect = { value ->
              val option =
                EditorValues.pageLayout.firstOrNull { it.value == value }
                  ?: return@EditorSettingsChipRow
              val currentMarginName =
                margins
                  .firstOrNull { margin ->
                    layout.pageMarginTop == margin.top &&
                      layout.pageMarginBottom == margin.bottom &&
                      layout.pageMarginLeft == margin.left &&
                      layout.pageMarginRight == margin.right
                  }
                  ?.value
              val newMargin =
                if (currentMarginName != null) {
                  option.margins.firstOrNull { it.value == currentMarginName }
                    ?: option.margins.first { it.value == "normal" }
                } else {
                  null
                }
              onLayoutChange(
                LayoutMode.Paginated(
                  pageWidth = option.layout.pageWidth,
                  pageHeight = option.layout.pageHeight,
                  pageMarginTop = newMargin?.top ?: layout.pageMarginTop,
                  pageMarginBottom = newMargin?.bottom ?: layout.pageMarginBottom,
                  pageMarginLeft = newMargin?.left ?: layout.pageMarginLeft,
                  pageMarginRight = newMargin?.right ?: layout.pageMarginRight,
                )
              )
            },
            trailing = {
              EditorSettingsTrailingChip(label = "사용자 정의", selected = currentPageSize == "custom") {
                sheet.present {
                  EditorPageLayoutSheet(layout = layout) { newLayout ->
                    scope.launch { onLayoutChange(newLayout) }
                  }
                }
              }
            },
          )
        }

        CardDivider()

        EditorSettingsRow(label = "여백") {
          EditorSettingsChipRow(
            options = margins.map { EditorOption(label = it.label, value = it.value) },
            selected = selectedMargin,
            onSelect = { value ->
              val margin = margins.firstOrNull { it.value == value } ?: return@EditorSettingsChipRow
              onLayoutChange(
                LayoutMode.Paginated(
                  pageWidth = layout.pageWidth,
                  pageHeight = layout.pageHeight,
                  pageMarginTop = margin.top,
                  pageMarginBottom = margin.bottom,
                  pageMarginLeft = margin.left,
                  pageMarginRight = margin.right,
                )
              )
            },
            trailing = {
              EditorSettingsTrailingChip(label = "사용자 정의", selected = selectedMargin == "custom") {
                sheet.present {
                  EditorPageLayoutSheet(layout = layout) { newLayout ->
                    scope.launch { onLayoutChange(newLayout) }
                  }
                }
              }
            },
          )
        }
      }

      is LayoutMode.Continuous -> {
        CardDivider()

        EditorSettingsRow(label = "본문 폭") {
          EditorSettingsChipRow(
            options = EditorValues.maxWidth,
            selected = layout.maxWidth,
            onSelect = { onLayoutChange(LayoutMode.Continuous(maxWidth = it)) },
          )
        }
      }
    }
  }
}

@Composable
internal fun EditorSettingsSectionDivider() {
  Spacer(modifier = Modifier.fillMaxWidth().height(12.dp).background(AppTheme.colors.surfaceInset))
}

@Composable
internal fun EditorSettingsSection(title: String, content: @Composable ColumnScope.() -> Unit) {
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
internal fun EditorSettingsRow(label: String, content: @Composable () -> Unit) {
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
internal fun EditorSettingsTrailingChip(
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
private fun EditorFontFamilyRow(
  style: EditorStyleSettings,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  sheet: Sheet,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  val selectableFontFamilies = fontFamilies.filter { it.isSelectableEditorSettingsFamily }
  val selectedFamily = selectableFontFamilies.firstOrNull { it.familyName == style.fontFamily }

  CardRow(
    onClick = {
      sheet.present {
        EditorFontFamilySheet(
          families = selectableFontFamilies,
          selectedFamilyName = style.fontFamily,
        ) { newFamily ->
          val availableWeights =
            selectableFontFamilies
              .firstOrNull { it.familyName == newFamily }
              ?.activeFonts
              ?.map { it.weight }
              ?.sorted()
              .orEmpty()
          val matchedWeight = matchWeight(availableWeights, style.fontWeight) ?: style.fontWeight
          onStyleChange(style.copy(fontFamily = newFamily, fontWeight = matchedWeight))
        }
      }
    }
  ) {
    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
      Text(text = "패밀리", style = AppTheme.typography.label, color = AppTheme.colors.textMuted)

      val representativeFont = selectedFamily?.representativeFont
      if (selectedFamily != null && representativeFont != null) {
        FontSpecimen(
          fontId = representativeFont.id,
          text = selectedFamily.displayName,
          fallbackTexts = listOf(selectedFamily.familyName),
          style = TextStyle(fontSize = 17.sp, fontWeight = FontWeight(representativeFont.weight)),
        )
      } else {
        Text(
          text = style.fontFamily,
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
}

@Composable
private fun EditorFontWeightRow(
  style: EditorStyleSettings,
  fontFamilies: List<EditorSettingsFontFamily_family>,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  val selectedFamily =
    fontFamilies
      .filter { it.isSelectableEditorSettingsFamily }
      .firstOrNull { it.familyName == style.fontFamily }
  val weightOptions =
    selectedFamily
      ?.activeFonts
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

  EditorSettingsRow(label = "굵기") {
    EditorSettingsChipRow(
      options = weightOptions,
      selected = style.fontWeight,
      onSelect = { onStyleChange(style.copy(fontWeight = it)) },
    )
  }
}

@Composable
private fun EditorFontSizeSlider(
  style: EditorStyleSettings,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSnapSlider(
    label = "크기",
    value = style.fontSize,
    onValueChange = { onStyleChange(style.copy(fontSize = it)) },
    range = 800..2400,
    sliderStep = 100,
    inputStep = 50,
    formatValue = { "${formatPointValue(it)}pt" },
    parseValue = { input -> input.trim().toDoubleOrNull()?.let { (it * 100).roundToInt() } },
    unitSuffix = "pt",
    fullRange = EditorValues.minFontSize..EditorValues.maxFontSize,
    modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
  )
}

@Composable
private fun EditorLetterSpacingSlider(
  style: EditorStyleSettings,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSnapSlider(
    label = "자간",
    value = style.letterSpacing,
    onValueChange = { onStyleChange(style.copy(letterSpacing = it)) },
    range = -10..40,
    sliderStep = 5,
    inputStep = 1,
    formatValue = { "$it%" },
    parseValue = { input -> input.trim().toDoubleOrNull()?.roundToInt() },
    unitSuffix = "%",
    modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
  )
}

@Composable
private fun EditorLineHeightSlider(
  style: EditorStyleSettings,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsSnapSlider(
    label = "행간",
    value = style.lineHeight,
    onValueChange = { onStyleChange(style.copy(lineHeight = it)) },
    range = 80..220,
    sliderStep = 10,
    inputStep = 1,
    formatValue = { "$it%" },
    parseValue = { input -> input.trim().toDoubleOrNull()?.roundToInt() },
    unitSuffix = "%",
    modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
  )
}

@Composable
private fun EditorParagraphIndentRow(
  style: EditorStyleSettings,
  label: String,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsRow(label = label) {
    EditorSettingsChipRow(
      options = EditorValues.paragraphIndent,
      selected = style.paragraphIndent,
      onSelect = { onStyleChange(style.copy(paragraphIndent = it)) },
    )
  }
}

@Composable
private fun EditorBlockGapRow(
  style: EditorStyleSettings,
  label: String,
  onStyleChange: suspend (EditorStyleSettings) -> Unit,
) {
  EditorSettingsRow(label = label) {
    EditorSettingsChipRow(
      options = EditorValues.blockGap,
      selected = style.blockGap,
      onSelect = { onStyleChange(style.copy(blockGap = it)) },
    )
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun EditorFontFamilySheet(
  families: List<EditorSettingsFontFamily_family>,
  selectedFamilyName: String,
  onSelect: suspend (String) -> Unit,
) {
  val selectedFamilyBringIntoViewRequester = remember { BringIntoViewRequester() }
  val selectedFamilyExists = families.any {
    it.familyName == selectedFamilyName && it.representativeFont != null
  }

  LaunchedEffect(selectedFamilyName, selectedFamilyExists) {
    if (selectedFamilyExists) {
      // Wait until SheetLayout establishes its constrained scroll viewport.
      withFrameNanos {}
      selectedFamilyBringIntoViewRequester.bringIntoView()
    }
  }

  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "폰트 패밀리",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
      SheetOptionList(items = families) { family ->
        val representativeFont = family.representativeFont ?: return@SheetOptionList
        val selected = selectedFamilyName == family.familyName

        SheetOptionRow(
          selected = selected,
          modifier =
            if (selected) {
              Modifier.bringIntoViewRequester(selectedFamilyBringIntoViewRequester)
            } else {
              Modifier
            },
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
}

private val EditorSettingsFontFamily_family.isSelectableEditorSettingsFamily: Boolean
  get() =
    state == FontFamilyState.ACTIVE &&
      source != FontFamilySource.FALLBACK &&
      activeFonts.isNotEmpty()

private val EditorSettingsFontFamily_family.activeFonts: List<EditorSettingsFontFamily_family.Font>
  get() = fonts.filter { it.state == FontState.ACTIVE }

private val EditorSettingsFontFamily_family.representativeFont:
  EditorSettingsFontFamily_family.Font?
  get() = activeFonts.reduceOrNull { previous, current ->
    val previousDiff = abs(previous.weight - 400)
    val currentDiff = abs(current.weight - 400)

    when {
      currentDiff < previousDiff -> current
      currentDiff == previousDiff && current.weight > previous.weight -> current
      else -> previous
    }
  }

private fun formatPointValue(value: Int): String {
  val whole = value / 100
  val fraction = abs(value % 100)
  if (fraction == 0) return whole.toString()
  return "$whole.${fraction.toString().padStart(2, '0')}".trimEnd('0').trimEnd('.')
}
