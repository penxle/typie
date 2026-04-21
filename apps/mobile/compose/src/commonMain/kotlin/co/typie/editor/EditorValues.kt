package co.typie.editor

data class EditorOption<T>(val label: String, val value: T)

fun <T> List<EditorOption<T>>.labelOf(value: T, fallback: () -> String): String =
  firstOrNull { it.value == value }?.label ?: fallback()

data class EditorColorOption(val label: String, val value: String, val themeKey: String?)

fun List<EditorColorOption>.labelOf(value: String, fallback: () -> String): String =
  firstOrNull { it.value == value }?.label ?: fallback()

data class EditorPageMarginOption(
  val label: String,
  val value: String,
  val top: Int,
  val bottom: Int,
  val left: Int,
  val right: Int,
)

data class EditorPageLayoutOption(
  val label: String,
  val value: String,
  val layout: PageLayout,
  val margins: List<EditorPageMarginOption>,
) {
  data class PageLayout(
    val pageWidth: Int,
    val pageHeight: Int,
    val pageMarginTop: Int,
    val pageMarginBottom: Int,
    val pageMarginLeft: Int,
    val pageMarginRight: Int,
  )
}

object EditorValues {
  val fontWeight =
    listOf(
      EditorOption(label = "가장 가늘게", value = 100),
      EditorOption(label = "아주 가늘게", value = 200),
      EditorOption(label = "가늘게", value = 300),
      EditorOption(label = "보통", value = 400),
      EditorOption(label = "중간", value = 500),
      EditorOption(label = "약간 굵게", value = 600),
      EditorOption(label = "굵게", value = 700),
      EditorOption(label = "아주 굵게", value = 800),
      EditorOption(label = "가장 굵게", value = 900),
    )

  val fontSize =
    listOf(
      EditorOption(label = "8", value = 800),
      EditorOption(label = "9", value = 900),
      EditorOption(label = "10", value = 1000),
      EditorOption(label = "11", value = 1100),
      EditorOption(label = "12", value = 1200),
      EditorOption(label = "14", value = 1400),
      EditorOption(label = "16", value = 1600),
      EditorOption(label = "18", value = 1800),
      EditorOption(label = "20", value = 2000),
      EditorOption(label = "22", value = 2200),
      EditorOption(label = "24", value = 2400),
      EditorOption(label = "30", value = 3000),
      EditorOption(label = "36", value = 3600),
      EditorOption(label = "48", value = 4800),
      EditorOption(label = "60", value = 6000),
      EditorOption(label = "72", value = 7200),
      EditorOption(label = "96", value = 9600),
    )

  val minFontSize = 100
  val maxFontSize = 20_000

  val textColor =
    listOf(
      EditorColorOption(label = "블랙", value = "black", themeKey = "text.black"),
      EditorColorOption(label = "다크 그레이", value = "darkgray", themeKey = "text.darkgray"),
      EditorColorOption(label = "그레이", value = "gray", themeKey = "text.gray"),
      EditorColorOption(label = "라이트 그레이", value = "lightgray", themeKey = "text.lightgray"),
      EditorColorOption(label = "화이트", value = "white", themeKey = "text.white"),
      EditorColorOption(label = "레드", value = "red", themeKey = "text.red"),
      EditorColorOption(label = "오렌지", value = "orange", themeKey = "text.orange"),
      EditorColorOption(label = "앰버", value = "amber", themeKey = "text.amber"),
      EditorColorOption(label = "옐로", value = "yellow", themeKey = "text.yellow"),
      EditorColorOption(label = "라임", value = "lime", themeKey = "text.lime"),
      EditorColorOption(label = "그린", value = "green", themeKey = "text.green"),
      EditorColorOption(label = "에메랄드", value = "emerald", themeKey = "text.emerald"),
      EditorColorOption(label = "틸", value = "teal", themeKey = "text.teal"),
      EditorColorOption(label = "시안", value = "cyan", themeKey = "text.cyan"),
      EditorColorOption(label = "스카이", value = "sky", themeKey = "text.sky"),
      EditorColorOption(label = "블루", value = "blue", themeKey = "text.blue"),
      EditorColorOption(label = "인디고", value = "indigo", themeKey = "text.indigo"),
      EditorColorOption(label = "바이올렛", value = "violet", themeKey = "text.violet"),
      EditorColorOption(label = "퍼플", value = "purple", themeKey = "text.purple"),
      EditorColorOption(label = "마젠타", value = "fuchsia", themeKey = "text.fuchsia"),
      EditorColorOption(label = "핑크", value = "pink", themeKey = "text.pink"),
      EditorColorOption(label = "로즈", value = "rose", themeKey = "text.rose"),
    )

  val textBackgroundColor =
    listOf(
      EditorColorOption(label = "배경 없음", value = "none", themeKey = null),
      EditorColorOption(label = "그레이", value = "gray", themeKey = "bg.gray"),
      EditorColorOption(label = "레드", value = "red", themeKey = "bg.red"),
      EditorColorOption(label = "오렌지", value = "orange", themeKey = "bg.orange"),
      EditorColorOption(label = "옐로", value = "yellow", themeKey = "bg.yellow"),
      EditorColorOption(label = "그린", value = "green", themeKey = "bg.green"),
      EditorColorOption(label = "블루", value = "blue", themeKey = "bg.blue"),
      EditorColorOption(label = "퍼플", value = "purple", themeKey = "bg.purple"),
    )

  val lineHeight =
    listOf(
      EditorOption(label = "80%", value = 80),
      EditorOption(label = "100%", value = 100),
      EditorOption(label = "120%", value = 120),
      EditorOption(label = "140%", value = 140),
      EditorOption(label = "160%", value = 160),
      EditorOption(label = "180%", value = 180),
      EditorOption(label = "200%", value = 200),
      EditorOption(label = "220%", value = 220),
    )

  val letterSpacing =
    listOf(
      EditorOption(label = "-10%", value = -10),
      EditorOption(label = "-5%", value = -5),
      EditorOption(label = "0%", value = 0),
      EditorOption(label = "5%", value = 5),
      EditorOption(label = "10%", value = 10),
      EditorOption(label = "20%", value = 20),
      EditorOption(label = "40%", value = 40),
    )

  val textAlign =
    listOf(
      EditorOption(label = "왼쪽 정렬", value = "left"),
      EditorOption(label = "가운데 정렬", value = "center"),
      EditorOption(label = "오른쪽 정렬", value = "right"),
      EditorOption(label = "양쪽 정렬", value = "justify"),
    )

  val paragraphIndent =
    listOf(
      EditorOption(label = "없음", value = 0),
      EditorOption(label = "0.5칸", value = 50),
      EditorOption(label = "1칸", value = 100),
      EditorOption(label = "2칸", value = 200),
    )

  val maxWidth =
    listOf(
      EditorOption(label = "400px", value = 400),
      EditorOption(label = "600px", value = 600),
      EditorOption(label = "800px", value = 800),
    )

  val blockGap =
    listOf(
      EditorOption(label = "없음", value = 0),
      EditorOption(label = "0.5줄", value = 50),
      EditorOption(label = "1줄", value = 100),
      EditorOption(label = "2줄", value = 200),
    )

  val pageLayout =
    listOf(
      EditorPageLayoutOption(
        label = "A4 (210mm × 297mm)",
        value = "a4",
        layout =
          EditorPageLayoutOption.PageLayout(
            pageWidth = 794,
            pageHeight = 1123,
            pageMarginTop = 94,
            pageMarginBottom = 94,
            pageMarginLeft = 94,
            pageMarginRight = 94,
          ),
        margins =
          listOf(
            EditorPageMarginOption(
              label = "좁게",
              value = "narrow",
              top = 57,
              bottom = 57,
              left = 57,
              right = 57,
            ),
            EditorPageMarginOption(
              label = "보통",
              value = "normal",
              top = 94,
              bottom = 94,
              left = 94,
              right = 94,
            ),
            EditorPageMarginOption(
              label = "넓게",
              value = "wide",
              top = 132,
              bottom = 132,
              left = 132,
              right = 132,
            ),
          ),
      ),
      EditorPageLayoutOption(
        label = "A5 (148mm × 210mm)",
        value = "a5",
        layout =
          EditorPageLayoutOption.PageLayout(
            pageWidth = 559,
            pageHeight = 794,
            pageMarginTop = 76,
            pageMarginBottom = 76,
            pageMarginLeft = 76,
            pageMarginRight = 76,
          ),
        margins =
          listOf(
            EditorPageMarginOption(
              label = "좁게",
              value = "narrow",
              top = 45,
              bottom = 45,
              left = 45,
              right = 45,
            ),
            EditorPageMarginOption(
              label = "보통",
              value = "normal",
              top = 76,
              bottom = 76,
              left = 76,
              right = 76,
            ),
            EditorPageMarginOption(
              label = "넓게",
              value = "wide",
              top = 106,
              bottom = 106,
              left = 106,
              right = 106,
            ),
          ),
      ),
      EditorPageLayoutOption(
        label = "B5 (176mm × 250mm)",
        value = "b5",
        layout =
          EditorPageLayoutOption.PageLayout(
            pageWidth = 665,
            pageHeight = 945,
            pageMarginTop = 57,
            pageMarginBottom = 57,
            pageMarginLeft = 57,
            pageMarginRight = 57,
          ),
        margins =
          listOf(
            EditorPageMarginOption(
              label = "좁게",
              value = "narrow",
              top = 38,
              bottom = 38,
              left = 38,
              right = 38,
            ),
            EditorPageMarginOption(
              label = "보통",
              value = "normal",
              top = 57,
              bottom = 57,
              left = 57,
              right = 57,
            ),
            EditorPageMarginOption(
              label = "넓게",
              value = "wide",
              top = 83,
              bottom = 83,
              left = 83,
              right = 83,
            ),
          ),
      ),
      EditorPageLayoutOption(
        label = "B6 (125mm × 176mm)",
        value = "b6",
        layout =
          EditorPageLayoutOption.PageLayout(
            pageWidth = 472,
            pageHeight = 665,
            pageMarginTop = 38,
            pageMarginBottom = 38,
            pageMarginLeft = 38,
            pageMarginRight = 38,
          ),
        margins =
          listOf(
            EditorPageMarginOption(
              label = "좁게",
              value = "narrow",
              top = 26,
              bottom = 26,
              left = 26,
              right = 26,
            ),
            EditorPageMarginOption(
              label = "보통",
              value = "normal",
              top = 38,
              bottom = 38,
              left = 38,
              right = 38,
            ),
            EditorPageMarginOption(
              label = "넓게",
              value = "wide",
              top = 57,
              bottom = 57,
              left = 57,
              right = 57,
            ),
          ),
      ),
    )
}
