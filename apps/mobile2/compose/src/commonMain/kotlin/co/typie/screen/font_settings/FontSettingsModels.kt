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

internal data class FontUploadSuccess(
  val familyId: String,
  val familyDisplayName: String,
  val weight: Int,
  val subfamilyDisplayName: String?,
)

internal data class FontUploadFailure(
  val name: String,
  val error: String,
)

internal enum class FontUploadSummaryStatus {
  Success,
  PartialSuccess,
  Failure,
}

internal data class FontUploadSummary(
  val status: FontUploadSummaryStatus,
  val title: String,
  val message: String,
  val successCount: Int,
  val failureCount: Int,
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

internal fun summarizeFontUploadResults(
  successes: List<FontUploadSuccess>,
  failures: List<FontUploadFailure>,
): FontUploadSummary? {
  if (successes.isEmpty() && failures.isEmpty()) return null

  val status = when {
    successes.isEmpty() -> FontUploadSummaryStatus.Failure
    failures.isEmpty() -> FontUploadSummaryStatus.Success
    else -> FontUploadSummaryStatus.PartialSuccess
  }

  val title = when (status) {
    FontUploadSummaryStatus.Success -> "폰트 업로드 완료"
    FontUploadSummaryStatus.PartialSuccess -> "폰트 업로드 일부 완료"
    FontUploadSummaryStatus.Failure -> "폰트 업로드 실패"
  }

  val sections = buildList {
    if (successes.isNotEmpty()) {
      val successesByFamily = linkedMapOf<String, MutableList<FontUploadSuccess>>()
      successes.forEach { success ->
        successesByFamily.getOrPut(success.familyId) { mutableListOf() }.add(success)
      }

      val successLines = successesByFamily.values.map { familyUploads ->
        val familyDisplayName = familyUploads.first().familyDisplayName
        val labels = familyUploads
          .sortedBy { it.weight }
          .map { fontWeightLabel(it.weight, it.subfamilyDisplayName) }
          .joinToString(", ")
        "• $familyDisplayName ($labels)"
      }

      add("${successes.size}개의 폰트가 추가되었어요.\n\n${successLines.joinToString("\n")}")
    }

    if (failures.isNotEmpty()) {
      val failureLines = failures.joinToString("\n") { failure -> "• ${failure.name}: ${failure.error}" }
      add("${failures.size}개의 폰트 업로드에 실패했어요.\n\n$failureLines")
    }
  }

  val note = if (status == FontUploadSummaryStatus.Success) {
    "업로드한 폰트는 이 화면에서 관리할 수 있어요."
  } else {
    null
  }

  val message = buildString {
    append(sections.joinToString("\n\n"))
    if (note != null) {
      append("\n\n")
      append(note)
    }
  }

  return FontUploadSummary(
    status = status,
    title = title,
    message = message,
    successCount = successes.size,
    failureCount = failures.size,
  )
}

internal fun fontSpecimenUrl(
  fontId: String,
  text: String,
): String {
  return "${Konfig.API_URL}/font/$fontId/specimen?text=${text.encodeURLQueryComponent()}"
}
