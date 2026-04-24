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
  val white = Color(0xFFFEFDF9)
  val black = Color(0xFF000000)

  val light =
    ThemeColors(
      gray =
        ColorScale(
          s50 = Color(0xFFFAF9F6),
          s100 = Color(0xFFF1F1EC),
          s200 = Color(0xFFE3E1D9),
          s300 = Color(0xFFD1CEC6),
          s400 = Color(0xFF9D9B93),
          s500 = Color(0xFF72716B),
          s600 = Color(0xFF53524D),
          s700 = Color(0xFF3E3E39),
          s800 = Color(0xFF2A2926),
          s900 = Color(0xFF22211E),
          s950 = Color(0xFF100F0D),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFFEFFFF6),
          s100 = Color(0xFFDAFFEA),
          s200 = Color(0xFFB9F5D4),
          s300 = Color(0xFF79E8B1),
          s400 = Color(0xFF00D185),
          s500 = Color(0xFF00A96D),
          s600 = Color(0xFF008857),
          s700 = Color(0xFF00714A),
          s800 = Color(0xFF005D3E),
          s900 = Color(0xFF004731),
          s950 = Color(0xFF003424),
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
          s50 = Color(0xFFF1F0EA),
          s100 = Color(0xFFE2E0D8),
          s200 = Color(0xFFC8C5BD),
          s300 = Color(0xFFB5B3A9),
          s400 = Color(0xFF91918D),
          s500 = Color(0xFF53534F),
          s600 = Color(0xFF3C3B38),
          s700 = Color(0xFF2D2C29),
          s800 = Color(0xFF22211D),
          s900 = Color(0xFF191816),
          s950 = Color(0xFF161513),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFF86D9B0),
          s100 = Color(0xFF45C992),
          s200 = Color(0xFF00C985),
          s300 = Color(0xFF00B56D),
          s400 = Color(0xFF009F58),
          s500 = Color(0xFF008447),
          s600 = Color(0xFF006638),
          s700 = Color(0xFF004A27),
          s800 = Color(0xFF002E1C),
          s900 = Color(0xFF001F13),
          s950 = Color(0xFF00150C),
        ),
      palette =
        ColorPalette(
          gray = Color(0xFF8C8A81),
          red = Color(0xFFD87F97),
          orange = Color(0xFFC99A58),
          yellow = Color(0xFFE0B76F),
          green = Color(0xFF5FA585),
          blue = Color(0xFF8490D2),
          purple = Color(0xFF9491C2),
        ),
    )
}
