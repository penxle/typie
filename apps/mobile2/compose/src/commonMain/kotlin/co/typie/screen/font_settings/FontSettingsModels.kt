package co.typie.screen.font_settings

import co.typie.Konfig
import io.ktor.http.encodeURLQueryComponent

internal data class FontSettingsFont(
  val id: String,
  val weight: Int,
  val subfamilyDisplayName: String? = null,
  val state: String = "ACTIVE",
)

internal data class FontSettingsFamily(
  val id: String,
  val familyName: String,
  val displayName: String,
  val source: String,
  val state: String,
  val fonts: List<FontSettingsFont>,
)

internal enum class FontUploadAction {
  PickFont,
  ShowSubscriptionNotice,
}

internal val FONT_WEIGHT_LABELS = mapOf(
  100 to "가장 가늘게",
  200 to "아주 가늘게",
  300 to "가늘게",
  400 to "보통",
  500 to "중간",
  600 to "약간 굵게",
  700 to "굵게",
  800 to "아주 굵게",
  900 to "가장 굵게",
)

internal fun uploadedFontFamilies(
  families: List<FontSettingsFamily>,
): List<FontSettingsFamily> {
  return families
    .filter { it.source == "USER" && it.state == "ACTIVE" }
    .map { family ->
      family.copy(
        fonts = family.fonts
          .filter { it.state == "ACTIVE" }
          .sortedBy { it.weight }
          .associateBy { it.weight }
          .values
          .toList(),
      )
    }
    .filter { it.fonts.isNotEmpty() }
}

internal fun representativeFont(
  fonts: List<FontSettingsFont>,
): FontSettingsFont? {
  if (fonts.isEmpty()) return null

  return fonts.reduce { previous, current ->
    val previousDiff = kotlin.math.abs(previous.weight - 400)
    val currentDiff = kotlin.math.abs(current.weight - 400)

    when {
      currentDiff < previousDiff -> current
      currentDiff == previousDiff && current.weight > previous.weight -> current
      else -> previous
    }
  }
}

internal fun fontWeightLabel(
  weight: Int,
  subfamilyDisplayName: String?,
): String {
  return FONT_WEIGHT_LABELS[weight]
    ?: subfamilyDisplayName?.takeIf { it.isNotBlank() }?.let { "$it ($weight)" }
    ?: weight.toString()
}

internal fun isSupportedTtfFontFile(
  filename: String,
  mimeType: String?,
): Boolean {
  val normalizedMimeType = mimeType?.substringBefore(';')?.lowercase()
  return filename.endsWith(".ttf", ignoreCase = true) ||
    normalizedMimeType == "font/ttf" ||
    normalizedMimeType == "application/x-font-ttf" ||
    normalizedMimeType == "application/x-truetype-font"
}

internal fun fontUploadAction(
  hasSubscription: Boolean,
): FontUploadAction {
  return if (hasSubscription) FontUploadAction.PickFont else FontUploadAction.ShowSubscriptionNotice
}

internal fun fontSpecimenUrl(
  fontId: String,
  text: String,
): String {
  return "${Konfig.API_URL}/font/$fontId/specimen?text=${text.encodeURLQueryComponent()}"
}
