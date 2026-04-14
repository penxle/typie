package co.typie.screen.settings.font_settings

import co.typie.subscription.SubscriptionServiceState

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

internal data class FontUploadProgress(val current: Int, val total: Int)

internal data class FontUploadSuccess(
  val familyId: String,
  val familyDisplayName: String,
  val weight: Int,
  val subfamilyDisplayName: String?,
)

internal enum class FontUploadError {
  UnsupportedFormat,
  InvalidFontStyle,
  UploadFailed,
  RefreshFailed,
}

internal data class FontUploadFailure(val name: String, val error: FontUploadError)

internal enum class FontUploadSummaryStatus {
  Success,
  PartialSuccess,
  Failure,
}

internal data class FontUploadSummary(
  val status: FontUploadSummaryStatus,
  val successes: List<FontUploadSuccess>,
  val failures: List<FontUploadFailure>,
)

internal enum class FontUploadAction {
  PickFont,
  ShowPlanUpgradeSheet,
}

internal val FONT_WEIGHT_LABELS =
  mapOf(
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

internal fun uploadedFontFamilies(families: List<FontSettingsFamily>): List<FontSettingsFamily> {
  return families
    .filter { it.source == "USER" && it.state == "ACTIVE" }
    .map { family ->
      family.copy(
        fonts =
          family.fonts
            .filter { it.state == "ACTIVE" }
            .sortedBy { it.weight }
            .associateBy { it.weight }
            .values
            .toList()
      )
    }
    .filter { it.fonts.isNotEmpty() }
}

internal fun representativeFont(fonts: List<FontSettingsFont>): FontSettingsFont? {
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

internal fun fontWeightLabel(weight: Int, subfamilyDisplayName: String?): String {
  return FONT_WEIGHT_LABELS[weight]
    ?: subfamilyDisplayName?.takeIf { it.isNotBlank() }?.let { "$it ($weight)" }
    ?: weight.toString()
}

internal fun isSupportedTtfFontFile(filename: String, mimeType: String?): Boolean {
  val normalizedMimeType = mimeType?.substringBefore(';')?.lowercase()
  return filename.endsWith(".ttf", ignoreCase = true) ||
    normalizedMimeType == "font/ttf" ||
    normalizedMimeType == "application/x-font-ttf" ||
    normalizedMimeType == "application/x-truetype-font"
}

internal fun fontUploadAction(state: SubscriptionServiceState): FontUploadAction? {
  return when (state) {
    is SubscriptionServiceState.Subscribed -> FontUploadAction.PickFont
    is SubscriptionServiceState.NotSubscribed -> FontUploadAction.ShowPlanUpgradeSheet
    is SubscriptionServiceState.Unknown -> null
  }
}

internal fun summarizeFontUploadResults(
  successes: List<FontUploadSuccess>,
  failures: List<FontUploadFailure>,
): FontUploadSummary? {
  if (successes.isEmpty() && failures.isEmpty()) return null

  val status =
    when {
      successes.isEmpty() -> FontUploadSummaryStatus.Failure
      failures.isEmpty() -> FontUploadSummaryStatus.Success
      else -> FontUploadSummaryStatus.PartialSuccess
    }

  return FontUploadSummary(status = status, successes = successes, failures = failures)
}
