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
  val white = Color(0xFFFEFDFA)
  val black = Color(0xFF000000)

  val light =
    ThemeColors(
      gray =
        ColorScale(
          s50 = Color(0xFFF9F9F7),
          s100 = Color(0xFFF1F0ED),
          s200 = Color(0xFFE2E1DB),
          s300 = Color(0xFFCFCEC8),
          s400 = Color(0xFF9C9B95),
          s500 = Color(0xFF71706D),
          s600 = Color(0xFF52514E),
          s700 = Color(0xFF3E3D3A),
          s800 = Color(0xFF292927),
          s900 = Color(0xFF21211F),
          s950 = Color(0xFF0F0F0D),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFFF9F9F7),
          s100 = Color(0xFFF1F0ED),
          s200 = Color(0xFFE2E1DB),
          s300 = Color(0xFFB6B4AD),
          s400 = Color(0xFF83827C),
          s500 = Color(0xFF71706D),
          s600 = Color(0xFF52514E),
          s700 = Color(0xFF3E3D3A),
          s800 = Color(0xFF292927),
          s900 = Color(0xFF21211F),
          s950 = Color(0xFF0F0F0D),
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
          s50 = Color(0xFFE4E3DF),
          s100 = Color(0xFFCCCBC6),
          s200 = Color(0xFFB1B0AA),
          s300 = Color(0xFF96958F),
          s400 = Color(0xFF797876),
          s500 = Color(0xFF535250),
          s600 = Color(0xFF393937),
          s700 = Color(0xFF292927),
          s800 = Color(0xFF1E1E1C),
          s900 = Color(0xFF181816),
          s950 = Color(0xFF121210),
        ),
      heatmap =
        ColorScale(
          s50 = Color(0xFFE4E3DF),
          s100 = Color(0xFFCCCBC6),
          s200 = Color(0xFFB1B0AA),
          s300 = Color(0xFF96958F),
          s400 = Color(0xFF797876),
          s500 = Color(0xFF6A6A67),
          s600 = Color(0xFF393937),
          s700 = Color(0xFF292927),
          s800 = Color(0xFF1E1E1C),
          s900 = Color(0xFF181816),
          s950 = Color(0xFF121210),
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
