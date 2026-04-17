package co.typie.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import co.typie.domain.bootstrap.BootstrapService
import co.typie.domain.bootstrap.BootstrapState
import co.typie.serialization.EnumSerializer
import co.typie.storage.Preference
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.HazeStyle
import dev.chrisbanes.haze.LocalHazeStyle
import kotlinx.serialization.Serializable

@Serializable(with = ThemeMode.Serializer::class)
enum class ThemeMode {
  System,
  Light,
  Dark;

  data object Serializer : EnumSerializer<ThemeMode>(entries, String::lowercase)
}

enum class ResolvedThemeMode {
  Light,
  Dark,
}

@Immutable
data class AppColors(
  // Text
  val textPrimary: Color,
  val textSecondary: Color,
  val textTertiary: Color,
  val textMuted: Color,

  // Surface
  val surfaceBase: Color,
  val surfaceDefault: Color,
  val surfaceSunken: Color,
  val surfaceRaised: Color,
  val surfaceTinted: Color,

  // Brand
  val brand: Color,
  val brandSubtle: Color,
  val textOnBrand: Color,
  val textOnBrandSubtle: Color,

  // Danger
  val danger: Color,
  val dangerSubtle: Color,
  val textOnDanger: Color,
  val textOnDangerSubtle: Color,

  // Success
  val success: Color,
  val successSubtle: Color,
  val textOnSuccess: Color,
  val textOnSuccessSubtle: Color,

  // Border
  val borderDefault: Color,
  val borderStrong: Color,
  val borderSubtle: Color,

  // Shadow
  val shadow: Color,
  val shadowAmbient: Color,

  // Skeleton
  val skeletonBone: Color,
  val skeletonHighlight: Color,

  // Utility
  val scrim: Color,

  // Palette
  val palette: ColorPalette,
)

val LightColors =
  AppColors(
    textPrimary = AppColor.light.gray.s900,
    textSecondary = AppColor.light.gray.s700,
    textTertiary = AppColor.light.gray.s500,
    textMuted = AppColor.light.gray.s400,
    surfaceBase = AppColor.light.gray.s50,
    surfaceDefault = AppColor.white,
    surfaceSunken = AppColor.light.gray.s50,
    surfaceRaised = AppColor.white,
    surfaceTinted = AppColor.light.gray.s100,
    brand = AppColor.light.brand.s500,
    brandSubtle = AppColor.light.brand.s100,
    textOnBrand = AppColor.white,
    textOnBrandSubtle = AppColor.light.brand.s700,
    danger = AppColor.light.red.s500,
    dangerSubtle = AppColor.light.red.s100,
    textOnDanger = AppColor.white,
    textOnDangerSubtle = AppColor.light.red.s500,
    success = AppColor.light.green.s400,
    successSubtle = AppColor.light.green.s50,
    textOnSuccess = AppColor.white,
    textOnSuccessSubtle = AppColor.light.green.s800,
    borderDefault = AppColor.light.gray.s200,
    borderStrong = AppColor.light.gray.s300,
    borderSubtle = AppColor.light.gray.s100,
    shadow = Color(0x1409090C),
    shadowAmbient = Color(0x0509090C),
    skeletonBone = Color(0xFFF8F8FC),
    skeletonHighlight = Color(0XFFf4F5FA),
    scrim = Color(0x52000000),
    palette = AppColor.light.palette,
  )

val DarkColors =
  AppColors(
    textPrimary = AppColor.dark.gray.s50,
    textSecondary = AppColor.dark.gray.s200,
    textTertiary = AppColor.dark.gray.s300,
    textMuted = AppColor.dark.gray.s400,
    surfaceBase = AppColor.dark.gray.s950,
    surfaceDefault = AppColor.dark.gray.s900,
    surfaceSunken = AppColor.dark.gray.s700,
    surfaceRaised = AppColor.dark.gray.s800,
    surfaceTinted = AppColor.dark.gray.s700,
    brand = AppColor.dark.brand.s400,
    brandSubtle = AppColor.dark.brand.s900,
    textOnBrand = AppColor.white,
    textOnBrandSubtle = AppColor.dark.brand.s100,
    danger = AppColor.dark.red.s300,
    dangerSubtle = AppColor.dark.red.s200,
    textOnDanger = AppColor.white,
    textOnDangerSubtle = AppColor.dark.red.s900,
    success = AppColor.dark.green.s200,
    successSubtle = AppColor.dark.green.s900,
    textOnSuccess = AppColor.white,
    textOnSuccessSubtle = AppColor.dark.green.s100,
    borderDefault = AppColor.dark.gray.s600,
    borderStrong = AppColor.dark.gray.s500,
    borderSubtle = AppColor.dark.gray.s800,
    shadow = Color(0x660A0B0E),
    shadowAmbient = Color(0x1A0A0B0E),
    skeletonBone = Color(0xFF16161A),
    skeletonHighlight = Color(0xFF1C1C20),
    scrim = Color(0x52000000),
    palette = AppColor.dark.palette,
  )

val LocalAppColors = staticCompositionLocalOf { LightColors }
val LocalHazeState = staticCompositionLocalOf { HazeState() }
val LocalThemeMode =
  compositionLocalOf<ResolvedThemeMode> {
    error("No ThemeMode provided. Wrap your content with AppTheme.")
  }

internal fun resolveThemeModeForStartup(
  startupState: BootstrapState,
  persistedThemeMode: ThemeMode,
): ThemeMode {
  return if (startupState is BootstrapState.Ready) {
    persistedThemeMode
  } else {
    ThemeMode.System
  }
}

internal fun resolveIsDarkTheme(themeMode: ThemeMode, systemIsDark: Boolean): Boolean {
  return when (themeMode) {
    ThemeMode.System -> systemIsDark
    ThemeMode.Light -> false
    ThemeMode.Dark -> true
  }
}

@Composable
fun AppTheme(content: @Composable () -> Unit) {
  val startupState = BootstrapService.state
  val isStartupReady = startupState is BootstrapState.Ready
  val persistedThemeMode = Preference.themeMode
  val themeMode =
    remember(isStartupReady) {
      mutableStateOf(resolveThemeModeForStartup(startupState, persistedThemeMode))
    }

  LaunchedEffect(isStartupReady) {
    if (!isStartupReady) return@LaunchedEffect
    snapshotFlow { Preference.themeMode }.collect { themeMode.value = it }
  }

  val isDark = resolveIsDarkTheme(themeMode = themeMode.value, systemIsDark = isSystemInDarkTheme())

  CompositionLocalProvider(
    LocalAppColors provides if (isDark) DarkColors else LightColors,
    LocalThemeMode provides if (isDark) ResolvedThemeMode.Dark else ResolvedThemeMode.Light,
    LocalHazeStyle provides HazeStyle(blurRadius = 20.dp, noiseFactor = 0f, tints = listOf()),
  ) {
    content()
  }
}

object AppTheme {
  val colors: AppColors
    @Composable get() = LocalAppColors.current

  val themeMode: ResolvedThemeMode
    @Composable get() = LocalThemeMode.current

  val typography: AppTypography
    get() = AppTypography

  val shapes: AppShapes
    get() = AppShapes

  val spacings: AppSpacings
    get() = AppSpacings
}
