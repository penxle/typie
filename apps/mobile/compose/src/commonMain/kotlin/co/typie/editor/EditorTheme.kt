package co.typie.editor

import androidx.compose.ui.graphics.Color
import co.typie.generated.resources.Res
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

enum class EditorThemeVariant(val key: String) {
  LightWhite("light-white"),
  LightSnow("light-snow"),
  LightButter("light-butter"),
  LightPeach("light-peach"),
  LightRose("light-rose"),
  LightLavender("light-lavender"),
  LightMint("light-mint"),
  LightLatte("light-latte"),
  DarkBlack("dark-black"),
  DarkCharcoal("dark-charcoal"),
  DarkGraphite("dark-graphite"),
  DarkMidnight("dark-midnight"),
  DarkNavy("dark-navy"),
  DarkObsidian("dark-obsidian"),
  DarkStorm("dark-storm"),
  DarkEspresso("dark-espresso");

  val isLight: Boolean
    get() = key.startsWith("light-")
}

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

  private val cache = mutableMapOf<EditorThemeVariant, ResolvedEditorTheme>()

  fun resolve(variant: EditorThemeVariant): ResolvedEditorTheme {
    return cache.getOrPut(variant) {
      val variantColors = data.variants.getValue(variant.key)
      val modeShared = if (variant.isLight) data.lightShared else data.darkShared
      val merged = data.shared + modeShared + variantColors
      ResolvedEditorTheme(merged.mapValues { (_, v) -> parseHexColor(v) })
    }
  }
}
