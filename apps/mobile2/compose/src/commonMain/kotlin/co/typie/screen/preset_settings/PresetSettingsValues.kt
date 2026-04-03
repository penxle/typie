package co.typie.screen.preset_settings

internal const val DEFAULT_FONT_FAMILY = "Pretendard"
internal const val DEFAULT_FONT_SIZE = 1200
internal const val DEFAULT_FONT_WEIGHT = 400
internal const val DEFAULT_TEXT_COLOR = "black"
internal const val DEFAULT_BACKGROUND_COLOR = "none"
internal const val DEFAULT_LETTER_SPACING = 0
internal const val DEFAULT_LINE_HEIGHT = 160
internal const val DEFAULT_MAX_WIDTH = 600
internal const val DEFAULT_PARAGRAPH_INDENT = 100
internal const val DEFAULT_BLOCK_GAP = 100
internal const val MIN_FONT_SIZE = 100
internal const val MAX_FONT_SIZE = 20_000

internal data class PresetOption<T>(
  val label: String,
  val value: T,
)

internal data class PageLayoutPresetOption(
  val label: String,
  val value: String,
  val layout: PresetLayout.Paginated,
)

internal val FONT_WEIGHT_OPTIONS = listOf(
  PresetOption(label = "가장 가늘게", value = 100),
  PresetOption(label = "아주 가늘게", value = 200),
  PresetOption(label = "가늘게", value = 300),
  PresetOption(label = "보통", value = 400),
  PresetOption(label = "중간", value = 500),
  PresetOption(label = "약간 굵게", value = 600),
  PresetOption(label = "굵게", value = 700),
  PresetOption(label = "아주 굵게", value = 800),
  PresetOption(label = "가장 굵게", value = 900),
)

internal val FONT_SIZE_OPTIONS = listOf(
  PresetOption(label = "8", value = 800),
  PresetOption(label = "9", value = 900),
  PresetOption(label = "10", value = 1000),
  PresetOption(label = "11", value = 1100),
  PresetOption(label = "12", value = 1200),
  PresetOption(label = "14", value = 1400),
  PresetOption(label = "16", value = 1600),
  PresetOption(label = "18", value = 1800),
  PresetOption(label = "20", value = 2000),
  PresetOption(label = "22", value = 2200),
  PresetOption(label = "24", value = 2400),
  PresetOption(label = "30", value = 3000),
  PresetOption(label = "36", value = 3600),
  PresetOption(label = "48", value = 4800),
  PresetOption(label = "60", value = 6000),
  PresetOption(label = "72", value = 7200),
  PresetOption(label = "96", value = 9600),
)

internal val TEXT_COLOR_OPTIONS = listOf(
  PresetOption(label = "블랙", value = "black"),
  PresetOption(label = "다크 그레이", value = "darkgray"),
  PresetOption(label = "그레이", value = "gray"),
  PresetOption(label = "라이트 그레이", value = "lightgray"),
  PresetOption(label = "화이트", value = "white"),
  PresetOption(label = "레드", value = "red"),
  PresetOption(label = "오렌지", value = "orange"),
  PresetOption(label = "앰버", value = "amber"),
  PresetOption(label = "옐로", value = "yellow"),
  PresetOption(label = "라임", value = "lime"),
  PresetOption(label = "그린", value = "green"),
  PresetOption(label = "에메랄드", value = "emerald"),
  PresetOption(label = "틸", value = "teal"),
  PresetOption(label = "시안", value = "cyan"),
  PresetOption(label = "스카이", value = "sky"),
  PresetOption(label = "블루", value = "blue"),
  PresetOption(label = "인디고", value = "indigo"),
  PresetOption(label = "바이올렛", value = "violet"),
  PresetOption(label = "퍼플", value = "purple"),
  PresetOption(label = "마젠타", value = "fuchsia"),
  PresetOption(label = "핑크", value = "pink"),
  PresetOption(label = "로즈", value = "rose"),
)

internal val BACKGROUND_COLOR_OPTIONS = listOf(
  PresetOption(label = "배경 없음", value = "none"),
  PresetOption(label = "그레이", value = "gray"),
  PresetOption(label = "레드", value = "red"),
  PresetOption(label = "오렌지", value = "orange"),
  PresetOption(label = "옐로", value = "yellow"),
  PresetOption(label = "그린", value = "green"),
  PresetOption(label = "블루", value = "blue"),
  PresetOption(label = "퍼플", value = "purple"),
)

internal val LETTER_SPACING_OPTIONS = listOf(
  PresetOption(label = "-10%", value = -10),
  PresetOption(label = "-5%", value = -5),
  PresetOption(label = "0%", value = 0),
  PresetOption(label = "5%", value = 5),
  PresetOption(label = "10%", value = 10),
  PresetOption(label = "20%", value = 20),
  PresetOption(label = "40%", value = 40),
)

internal val LINE_HEIGHT_OPTIONS = listOf(
  PresetOption(label = "80%", value = 80),
  PresetOption(label = "100%", value = 100),
  PresetOption(label = "120%", value = 120),
  PresetOption(label = "140%", value = 140),
  PresetOption(label = "160%", value = 160),
  PresetOption(label = "180%", value = 180),
  PresetOption(label = "200%", value = 200),
  PresetOption(label = "220%", value = 220),
)

internal val PARAGRAPH_INDENT_OPTIONS = listOf(
  PresetOption(label = "없음", value = 0),
  PresetOption(label = "0.5칸", value = 50),
  PresetOption(label = "1칸", value = 100),
  PresetOption(label = "2칸", value = 200),
)

internal val MAX_WIDTH_OPTIONS = listOf(
  PresetOption(label = "400px", value = 400),
  PresetOption(label = "600px", value = 600),
  PresetOption(label = "800px", value = 800),
)

internal val BLOCK_GAP_OPTIONS = listOf(
  PresetOption(label = "없음", value = 0),
  PresetOption(label = "0.5줄", value = 50),
  PresetOption(label = "1줄", value = 100),
  PresetOption(label = "2줄", value = 200),
)

internal val PAGE_LAYOUT_OPTIONS = listOf(
  PageLayoutPresetOption(
    label = "A4 (210mm × 297mm)",
    value = "a4",
    layout = PresetLayout.Paginated(
      pageWidth = 794,
      pageHeight = 1123,
      pageMarginTop = 94,
      pageMarginBottom = 94,
      pageMarginLeft = 94,
      pageMarginRight = 94,
    ),
  ),
  PageLayoutPresetOption(
    label = "A5 (148mm × 210mm)",
    value = "a5",
    layout = PresetLayout.Paginated(
      pageWidth = 559,
      pageHeight = 794,
      pageMarginTop = 76,
      pageMarginBottom = 76,
      pageMarginLeft = 76,
      pageMarginRight = 76,
    ),
  ),
  PageLayoutPresetOption(
    label = "B5 (176mm × 250mm)",
    value = "b5",
    layout = PresetLayout.Paginated(
      pageWidth = 665,
      pageHeight = 945,
      pageMarginTop = 57,
      pageMarginBottom = 57,
      pageMarginLeft = 57,
      pageMarginRight = 57,
    ),
  ),
  PageLayoutPresetOption(
    label = "B6 (125mm × 176mm)",
    value = "b6",
    layout = PresetLayout.Paginated(
      pageWidth = 472,
      pageHeight = 665,
      pageMarginTop = 38,
      pageMarginBottom = 38,
      pageMarginLeft = 38,
      pageMarginRight = 38,
    ),
  ),
)

internal fun mmToPx(mm: Int): Int {
  return kotlin.math.round((mm * 96.0) / 25.4).toInt()
}

internal fun pxToMm(px: Int): Int {
  return kotlin.math.round((px * 25.4) / 96.0).toInt()
}

internal fun createPaginatedLayout(preset: String = "a4"): PresetLayout.Paginated {
  return PAGE_LAYOUT_OPTIONS.firstOrNull { it.value == preset }?.layout
    ?: PAGE_LAYOUT_OPTIONS.first().layout
}

private val MIN_CONTENT_SIZE_PX = mmToPx(50)

internal fun getMaxMargin(
  side: PageMarginSide,
  layout: PresetLayout.Paginated,
): Int {
  return when (side) {
    PageMarginSide.Left -> maxOf(0, layout.pageWidth - layout.pageMarginRight - MIN_CONTENT_SIZE_PX)
    PageMarginSide.Right -> maxOf(0, layout.pageWidth - layout.pageMarginLeft - MIN_CONTENT_SIZE_PX)
    PageMarginSide.Top -> maxOf(0, layout.pageHeight - layout.pageMarginBottom - MIN_CONTENT_SIZE_PX)
    PageMarginSide.Bottom -> maxOf(0, layout.pageHeight - layout.pageMarginTop - MIN_CONTENT_SIZE_PX)
  }
}

internal fun formatPresetPointValue(value: Int): String {
  val whole = value / 100
  val fraction = kotlin.math.abs(value % 100)
  if (fraction == 0) return whole.toString()
  return "$whole.${fraction.toString().padStart(2, '0')}".trimEnd('0').trimEnd('.')
}
