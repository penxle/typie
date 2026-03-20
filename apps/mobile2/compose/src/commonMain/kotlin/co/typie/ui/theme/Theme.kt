package co.typie.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

enum class ThemeMode { System, Light, Dark }

@Immutable
data class AppColors(
    val isDark: Boolean,

    // Text
    val textDefault: Color,
    val textSubtle: Color,
    val textMuted: Color,
    val textFaint: Color,
    val textDisabled: Color,
    val textBright: Color,
    val textDanger: Color,
    val textSuccess: Color,
    val textLink: Color,
    val textBrand: Color,

    // Surface
    val surfaceDefault: Color,
    val surfaceSubtle: Color,
    val surfaceMuted: Color,
    val surfaceDark: Color,

    // Interactive
    val interactiveHover: Color,
    val interactiveDisabled: Color,

    // Accent — Brand
    val accentBrand: Color,
    val accentBrandHover: Color,
    val accentBrandActive: Color,
    val accentBrandSubtle: Color,

    // Accent — Info
    val accentInfo: Color,
    val accentInfoSubtle: Color,

    // Accent — Danger
    val accentDanger: Color,
    val accentDangerHover: Color,
    val accentDangerActive: Color,
    val accentDangerSubtle: Color,

    // Accent — Warning
    val accentWarning: Color,
    val accentWarningSubtle: Color,

    // Accent — Success
    val accentSuccess: Color,
    val accentSuccessSubtle: Color,

    // Border
    val borderDefault: Color,
    val borderStrong: Color,
    val borderSubtle: Color,
    val borderBrand: Color,
    val borderDanger: Color,

    // Shadow
    val shadowDefault: Color,
)

val LightColors = AppColors(
    isDark = false,
    textDefault = AppColor.light.gray.s900,
    textSubtle = AppColor.light.gray.s700,
    textMuted = AppColor.light.gray.s600,
    textFaint = AppColor.light.gray.s500,
    textDisabled = AppColor.light.gray.s400,
    textBright = AppColor.white,
    textDanger = AppColor.light.red.s500,
    textSuccess = AppColor.light.green.s700,
    textLink = AppColor.light.blue.s600,
    textBrand = AppColor.light.brand.s500,

    surfaceDefault = AppColor.white,
    surfaceSubtle = AppColor.light.gray.s50,
    surfaceMuted = AppColor.light.gray.s100,
    surfaceDark = AppColor.light.gray.s700,

    interactiveHover = AppColor.light.gray.s200,
    interactiveDisabled = AppColor.light.gray.s200,

    accentBrand = AppColor.light.brand.s500,
    accentBrandHover = AppColor.light.brand.s600,
    accentBrandActive = AppColor.light.brand.s700,
    accentBrandSubtle = AppColor.light.brand.s100,

    accentInfo = AppColor.light.blue.s500,
    accentInfoSubtle = AppColor.light.blue.s50,

    accentDanger = AppColor.light.red.s600,
    accentDangerHover = AppColor.light.red.s500,
    accentDangerActive = AppColor.light.red.s700,
    accentDangerSubtle = AppColor.light.red.s50,

    accentWarning = AppColor.light.amber.s600,
    accentWarningSubtle = AppColor.light.amber.s50,

    accentSuccess = AppColor.light.green.s700,
    accentSuccessSubtle = AppColor.light.green.s50,

    borderDefault = AppColor.light.gray.s200,
    borderStrong = AppColor.light.gray.s300,
    borderSubtle = AppColor.light.gray.s100,
    borderBrand = AppColor.light.brand.s600,
    borderDanger = AppColor.light.red.s600,

    shadowDefault = AppColor.light.gray.s950,
)

val DarkColors = AppColors(
    isDark = true,
    textDefault = AppColor.dark.gray.s50,
    textSubtle = AppColor.dark.gray.s100,
    textMuted = AppColor.dark.gray.s200,
    textFaint = AppColor.dark.gray.s300,
    textDisabled = AppColor.dark.gray.s400,
    textBright = AppColor.dark.gray.s50,
    textDanger = AppColor.dark.red.s300,
    textSuccess = AppColor.dark.green.s300,
    textLink = AppColor.dark.blue.s400,
    textBrand = AppColor.dark.brand.s300,

    surfaceDefault = AppColor.dark.gray.s900,
    surfaceSubtle = AppColor.dark.gray.s800,
    surfaceMuted = AppColor.dark.gray.s700,
    surfaceDark = AppColor.dark.gray.s700,

    interactiveHover = AppColor.dark.gray.s600,
    interactiveDisabled = AppColor.dark.gray.s800,

    accentBrand = AppColor.dark.brand.s400,
    accentBrandHover = AppColor.dark.brand.s500,
    accentBrandActive = AppColor.dark.brand.s600,
    accentBrandSubtle = AppColor.dark.brand.s900,

    accentInfo = AppColor.dark.blue.s200,
    accentInfoSubtle = AppColor.dark.blue.s900,

    accentDanger = AppColor.dark.red.s400,
    accentDangerHover = AppColor.dark.red.s500,
    accentDangerActive = AppColor.dark.red.s600,
    accentDangerSubtle = AppColor.dark.red.s900,

    accentWarning = AppColor.dark.amber.s300,
    accentWarningSubtle = AppColor.dark.amber.s900,

    accentSuccess = AppColor.dark.green.s300,
    accentSuccessSubtle = AppColor.dark.green.s900,

    borderDefault = AppColor.dark.gray.s700,
    borderStrong = AppColor.dark.gray.s600,
    borderSubtle = AppColor.dark.gray.s800,
    borderBrand = AppColor.dark.brand.s400,
    borderDanger = AppColor.dark.red.s400,

    shadowDefault = AppColor.dark.gray.s950,
)

val LocalAppColors = staticCompositionLocalOf { LightColors }
val LocalThemeMode = compositionLocalOf<MutableState<ThemeMode>> {
    error("No ThemeMode provided. Wrap your content with AppTheme.")
}

@Composable
fun AppTheme(content: @Composable () -> Unit) {
    val themeMode = remember { mutableStateOf(ThemeMode.System) }
    val isDark = when (themeMode.value) {
        ThemeMode.System -> isSystemInDarkTheme()
        ThemeMode.Light -> false
        ThemeMode.Dark -> true
    }
    CompositionLocalProvider(
        LocalAppColors provides if (isDark) DarkColors else LightColors,
        LocalThemeMode provides themeMode,
    ) {
        content()
    }
}

object AppTheme {
    val colors: AppColors
        @Composable get() = LocalAppColors.current

    val themeMode: ThemeMode
        @Composable get() = LocalThemeMode.current.value
}
