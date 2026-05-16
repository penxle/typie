package co.typie.editor

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import co.typie.editor.ffi.ThemeVariant
import co.typie.generated.resources.Res
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

@Serializable
data class EditorThemeData(
  val shared: Map<String, String>,
  val lightShared: Map<String, String>,
  val darkShared: Map<String, String>,
  val variants: Map<String, Map<String, String>>,
)

val ThemeVariant.key: String
  get() =
    when (this) {
      ThemeVariant.LightWhite -> "light-white"
      ThemeVariant.LightSnow -> "light-snow"
      ThemeVariant.LightButter -> "light-butter"
      ThemeVariant.LightPeach -> "light-peach"
      ThemeVariant.LightRose -> "light-rose"
      ThemeVariant.LightLavender -> "light-lavender"
      ThemeVariant.LightMint -> "light-mint"
      ThemeVariant.LightLatte -> "light-latte"
      ThemeVariant.DarkBlack -> "dark-black"
      ThemeVariant.DarkCharcoal -> "dark-charcoal"
      ThemeVariant.DarkGraphite -> "dark-graphite"
      ThemeVariant.DarkMidnight -> "dark-midnight"
      ThemeVariant.DarkNavy -> "dark-navy"
      ThemeVariant.DarkObsidian -> "dark-obsidian"
      ThemeVariant.DarkStorm -> "dark-storm"
      ThemeVariant.DarkEspresso -> "dark-espresso"
    }

private val ThemeVariant.isLight: Boolean
  get() = key.startsWith("light-")

@Composable
fun currentEditorThemeVariant(): ThemeVariant =
  if (AppTheme.themeMode == ResolvedThemeMode.Dark) ThemeVariant.DarkBlack
  else ThemeVariant.LightWhite

data class ResolvedEditorTheme(val colors: Map<String, Color>) {
  operator fun get(key: String): Color? = colors[key]
}

private fun parseHexColor(hex: String): Color {
  val sanitized = hex.removePrefix("#")
  val argb = if (sanitized.length == 6) "FF$sanitized" else sanitized
  return Color(argb.toLong(16))
}

object EditorTheme {
  private val data: EditorThemeData by lazy {
    runBlocking {
      Json.decodeFromString<EditorThemeData>(
        Res.readBytes("files/editor/theme.json").decodeToString()
      )
    }
  }

  private val cache = mutableMapOf<ThemeVariant, ResolvedEditorTheme>()

  fun resolve(variant: ThemeVariant): ResolvedEditorTheme {
    return cache.getOrPut(variant) {
      val variantColors = data.variants.getValue(variant.key)
      val modeShared = if (variant.isLight) data.lightShared else data.darkShared
      val merged = data.shared + modeShared + variantColors
      ResolvedEditorTheme(merged.mapValues { (_, v) -> parseHexColor(v) })
    }
  }
}
