package co.typie.screen.settings.fontsettings

internal enum class FontUploadError {
  Generic,
  InvalidFontStyle,
}

internal data class FontUploadProgress(val current: Int, val total: Int)

internal data class FontUploadSuccess(
  val familyId: String,
  val familyDisplayName: String,
  val weight: Int,
  val subfamilyDisplayName: String?,
)

internal data class FontUploadFailure(val name: String, val error: FontUploadError)

internal enum class FontUploadStatus {
  Success,
  PartialSuccess,
  Failure,
}

internal data class FontUploadResult(
  val status: FontUploadStatus,
  val successes: List<FontUploadSuccess>,
  val failures: List<FontUploadFailure>,
)

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

internal fun fontWeightLabel(weight: Int, subfamilyDisplayName: String?): String {
  return FONT_WEIGHT_LABELS[weight]
    ?: subfamilyDisplayName?.takeIf { it.isNotBlank() }?.let { "$it ($weight)" }
    ?: weight.toString()
}
