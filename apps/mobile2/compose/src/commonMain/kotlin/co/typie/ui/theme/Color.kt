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
          s50 = Color(0xFFFAF9F6),
          s100 = Color(0xFFF1F1EC),
          s200 = Color(0xFFE3E1D9),
          s300 = Color(0xFFB7B5AB),
          s400 = Color(0xFF858279),
          s500 = Color(0xFF72716B),
          s600 = Color(0xFF53524D),
          s700 = Color(0xFF3E3E39),
          s800 = Color(0xFF2A2926),
          s900 = Color(0xFF22211E),
          s950 = Color(0xFF100F0D),
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
          s50 = Color(0xFFE5E4DE),
          s100 = Color(0xFFCDCBC4),
          s200 = Color(0xFFB3B0A8),
          s300 = Color(0xFF97958D),
          s400 = Color(0xFF797975),
          s500 = Color(0xFF53534F),
          s600 = Color(0xFF3A3936),
          s700 = Color(0xFF2A2926),
          s800 = Color(0xFF1F1E1B),
          s900 = Color(0xFF191816),
          s950 = Color(0xFF131210),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFFE5E4DE),
          s100 = Color(0xFFCDCBC4),
          s200 = Color(0xFFB3B0A8),
          s300 = Color(0xFF97958D),
          s400 = Color(0xFF797975),
          s500 = Color(0xFF6B6B66),
          s600 = Color(0xFF3A3936),
          s700 = Color(0xFF2A2926),
          s800 = Color(0xFF1F1E1B),
          s900 = Color(0xFF191816),
          s950 = Color(0xFF131210),
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
