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
  val textDefault: Color,
  val textMuted: Color,
  val textHint: Color,
  val textOnInverse: Color,

  // Surface
  val surfaceCanvas: Color,
  val surfaceDefault: Color,
  val surfaceInset: Color,
  val surfaceInverse: Color,

  // Border
  val borderEmphasis: Color,
  val borderDefault: Color,
  val borderHairline: Color,

  // Signal
  val danger: Color,
  val dangerSubtle: Color,
  val textOnDanger: Color,
  val textOnDangerSubtle: Color,
  val success: Color,
  val successSubtle: Color,
  val textOnSuccess: Color,
  val textOnSuccessSubtle: Color,

  // Skeleton
  val skeletonBase: Color,
  val skeletonShimmer: Color,
  val skeletonBaseInverse: Color,
  val skeletonShimmerInverse: Color,

  // Utility
  val scrim: Color,

  // Palette
  val palette: ColorPalette,
)

val LightColors =
  AppColors(
    textDefault = AppColor.light.gray.s900,
    textMuted = AppColor.light.gray.s600,
    textHint = AppColor.light.gray.s500,
    textOnInverse = AppColor.white,
    surfaceCanvas = AppColor.light.gray.s50,
    surfaceDefault = AppColor.white,
    surfaceInset = AppColor.light.gray.s100,
    surfaceInverse = AppColor.light.gray.s900,
    borderEmphasis = AppColor.light.gray.s300,
    borderDefault = AppColor.light.gray.s200,
    borderHairline = AppColor.light.gray.s100,
    danger = Color(0xFFD32055),
    dangerSubtle = Color(0xFFF7DDE1),
    textOnDanger = AppColor.white,
    textOnDangerSubtle = Color(0xFF7A1E35),
    success = Color(0xFF137D56),
    successSubtle = Color(0xFFD8F3E2),
    textOnSuccess = AppColor.white,
    textOnSuccessSubtle = Color(0xFF1A553D),
    skeletonBase = AppColor.light.gray.s100,
    skeletonShimmer = Color(0xFFE9E8DF),
    skeletonBaseInverse = AppColor.light.gray.s800,
    skeletonShimmerInverse = AppColor.light.gray.s700,
    scrim = Color(0x521A180E),
    palette = AppColor.light.palette,
  )

val DarkColors =
  AppColors(
    textDefault = AppColor.dark.gray.s50,
    textMuted = AppColor.dark.gray.s300,
    textHint = AppColor.dark.gray.s400,
    textOnInverse = AppColor.dark.gray.s950,
    surfaceCanvas = AppColor.dark.gray.s950,
    surfaceDefault = AppColor.dark.gray.s900,
    surfaceInset = AppColor.dark.gray.s800,
    surfaceInverse = AppColor.dark.gray.s50,
    borderEmphasis = AppColor.dark.gray.s600,
    borderDefault = AppColor.dark.gray.s700,
    borderHairline = AppColor.dark.gray.s800,
    danger = Color(0xFFE06680),
    dangerSubtle = Color(0xFF3F1B28),
    textOnDanger = AppColor.dark.gray.s900,
    textOnDangerSubtle = Color(0xFFDEB6BF),
    success = Color(0xFF3F8A66),
    successSubtle = Color(0xFF1A3D2C),
    textOnSuccess = Color(0xFF040404),
    textOnSuccessSubtle = Color(0xFFBFDACA),
    skeletonBase = AppColor.dark.gray.s800,
    skeletonShimmer = Color(0xFF292824),
    skeletonBaseInverse = AppColor.dark.gray.s100,
    skeletonShimmerInverse = AppColor.dark.gray.s200,
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
    LocalAppShadows provides if (isDark) DarkAppShadows else LightAppShadows,
    LocalThemeMode provides if (isDark) ResolvedThemeMode.Dark else ResolvedThemeMode.Light,
    LocalHazeStyle provides HazeStyle(blurRadius = 20.dp, noiseFactor = 0f, tints = listOf()),
  ) {
    content()
  }
}

object AppTheme {
  val colors: AppColors
    @Composable get() = LocalAppColors.current

  val shadows: AppShadows
    @Composable get() = LocalAppShadows.current

  val themeMode: ResolvedThemeMode
    @Composable get() = LocalThemeMode.current

  val typography: AppTypography
    get() = AppTypography

  val shapes: AppShapes
    get() = AppShapes

  val spacings: AppSpacings
    get() = AppSpacings
}
