package co.typie.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
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
