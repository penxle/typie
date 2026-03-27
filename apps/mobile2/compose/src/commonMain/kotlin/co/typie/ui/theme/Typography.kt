package co.typie.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import co.typie.generated.resources.Res
import co.typie.generated.resources.suit_bold
import co.typie.generated.resources.suit_extrabold
import co.typie.generated.resources.suit_extralight
import co.typie.generated.resources.suit_heavy
import co.typie.generated.resources.suit_light
import co.typie.generated.resources.suit_medium
import co.typie.generated.resources.suit_regular
import co.typie.generated.resources.suit_semibold
import co.typie.generated.resources.suit_thin
import org.jetbrains.compose.resources.Font

val SuitFontFamily: FontFamily
  @Composable get() = FontFamily(
    Font(Res.font.suit_thin, FontWeight.Thin),
    Font(Res.font.suit_extralight, FontWeight.ExtraLight),
    Font(Res.font.suit_light, FontWeight.Light),
    Font(Res.font.suit_regular, FontWeight.Normal),
    Font(Res.font.suit_medium, FontWeight.Medium),
    Font(Res.font.suit_semibold, FontWeight.SemiBold),
    Font(Res.font.suit_bold, FontWeight.Bold),
    Font(Res.font.suit_extrabold, FontWeight.ExtraBold),
    Font(Res.font.suit_heavy, FontWeight.Black),
  )

object AppTypography {
  /** 화면 최상단 대제목 (홈, 더 보기 등) */
  val display: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 28.sp,
        fontWeight = FontWeight.W600,
        lineHeight = 36.sp,
      )
    }

  /** 섹션/다이얼로그 제목 */
  val heading: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 22.sp,
        fontWeight = FontWeight.W600,
        lineHeight = 28.sp,
      )
    }

  /** 카드 제목, TopBar 타이틀, 리스트 주요 텍스트 */
  val title: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 17.sp,
        fontWeight = FontWeight.W600,
        lineHeight = 22.sp,
      )
    }

  /** 메뉴 항목, 카드 row 레이블, 폼 필드 */
  val label: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 15.sp,
        fontWeight = FontWeight.W600,
        lineHeight = 20.sp,
      )
    }

  /** 본문, 기본 텍스트 */
  val body: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 16.sp,
        fontWeight = FontWeight.W400,
        lineHeight = 24.sp,
      )
    }

  /** 버튼, 링크, 탭, 폼 액션 */
  val action: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 15.sp,
        fontWeight = FontWeight.W500,
        lineHeight = 20.sp,
      )
    }

  /** 헬퍼 텍스트, 플레이스홀더, 타임스탬프. 서브타이틀 */
  val caption: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 13.sp,
        fontWeight = FontWeight.W400,
        lineHeight = 18.sp,
      )
    }

  /** 배지 카운트, 인라인 태그, 법적 고지문 */
  val micro: TextStyle
    @Composable get() {
      val fontFamily = rememberSuitFontFamily()
      return TextStyle(
        fontFamily = fontFamily,
        fontSize = 11.sp,
        fontWeight = FontWeight.W400,
        lineHeight = 16.sp,
      )
    }
}

@Composable
private fun rememberSuitFontFamily(): FontFamily {
  val thin = Font(Res.font.suit_thin, FontWeight.Thin)
  val extraLight = Font(Res.font.suit_extralight, FontWeight.ExtraLight)
  val light = Font(Res.font.suit_light, FontWeight.Light)
  val regular = Font(Res.font.suit_regular, FontWeight.Normal)
  val medium = Font(Res.font.suit_medium, FontWeight.Medium)
  val semiBold = Font(Res.font.suit_semibold, FontWeight.SemiBold)
  val bold = Font(Res.font.suit_bold, FontWeight.Bold)
  val extraBold = Font(Res.font.suit_extrabold, FontWeight.ExtraBold)
  val heavy = Font(Res.font.suit_heavy, FontWeight.Black)
  return remember(thin, extraLight, light, regular, medium, semiBold, bold, extraBold, heavy) {
    FontFamily(thin, extraLight, light, regular, medium, semiBold, bold, extraBold, heavy)
  }
}
