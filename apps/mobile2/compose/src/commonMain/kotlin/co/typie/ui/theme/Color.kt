package co.typie.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

@Immutable
data class ColorScale(
  val s50: Color,
  val s100: Color,
  val s200: Color,
  val s300: Color,
  val s400: Color,
  val s500: Color,
  val s600: Color,
  val s700: Color,
  val s800: Color,
  val s900: Color,
  val s950: Color,
)

@Immutable
data class ColorPalette(
  val gray: Color,
  val red: Color,
  val orange: Color,
  val yellow: Color,
  val green: Color,
  val blue: Color,
  val purple: Color,
)

@Immutable
data class ThemeColors(val gray: ColorScale, val heatmap: ColorScale, val palette: ColorPalette)

object AppColor {
  val white = Color(0xFFFFFFFF)
  val black = Color(0xFF000000)

  val light =
    ThemeColors(
      gray =
        ColorScale(
          s50 = Color(0xFFFAF9F5),
          s100 = Color(0xFFF2F1EB),
          s200 = Color(0xFFE4E2D7),
          s300 = Color(0xFFD2CFC4),
          s400 = Color(0xFF9E9C91),
          s500 = Color(0xFF73716A),
          s600 = Color(0xFF54524C),
          s700 = Color(0xFF3F3E38),
          s800 = Color(0xFF2A2925),
          s900 = Color(0xFF22211D),
          s950 = Color(0xFF100F0C),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFFFAF9F5),
          s100 = Color(0xFFF2F1EB),
          s200 = Color(0xFFE4E2D7),
          s300 = Color(0xFFB9B6A8),
          s400 = Color(0xFF868377),
          s500 = Color(0xFF73716A),
          s600 = Color(0xFF54524C),
          s700 = Color(0xFF3F3E38),
          s800 = Color(0xFF2A2925),
          s900 = Color(0xFF22211D),
          s950 = Color(0xFF100F0C),
        ),
      palette =
        ColorPalette(
          gray = Color(0xFF78766D),
          red = Color(0xFFD65775),
          orange = Color(0xFFBB7E2C),
          yellow = Color(0xFFE0AA43),
          green = Color(0xFF37996F),
          blue = Color(0xFF6473CF),
          purple = Color(0xFF7A78BA),
        ),
    )

  val dark =
    ThemeColors(
      gray =
        ColorScale(
          s50 = Color(0xFFE6E4DC),
          s100 = Color(0xFFCECCC2),
          s200 = Color(0xFFB4B1A6),
          s300 = Color(0xFF98968B),
          s400 = Color(0xFF7A7974),
          s500 = Color(0xFF54534E),
          s600 = Color(0xFF3A3935),
          s700 = Color(0xFF2A2925),
          s800 = Color(0xFF1F1E1A),
          s900 = Color(0xFF191815),
          s950 = Color(0xFF13120F),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFFE6E4DC),
          s100 = Color(0xFFCECCC2),
          s200 = Color(0xFFB4B1A6),
          s300 = Color(0xFF98968B),
          s400 = Color(0xFF7A7974),
          s500 = Color(0xFF6C6B65),
          s600 = Color(0xFF3A3935),
          s700 = Color(0xFF2A2925),
          s800 = Color(0xFF1F1E1A),
          s900 = Color(0xFF191815),
          s950 = Color(0xFF13120F),
        ),
      palette =
        ColorPalette(
          gray = Color(0xFF8C8A81),
          red = Color(0xFFD87F97),
          orange = Color(0xFFC99A58),
          yellow = Color(0xFFD5BC6E),
          green = Color(0xFF5FA585),
          blue = Color(0xFF8490D2),
          purple = Color(0xFF9491C2),
        ),
    )
}
