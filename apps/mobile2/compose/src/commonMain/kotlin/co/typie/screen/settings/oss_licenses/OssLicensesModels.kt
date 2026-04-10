package co.typie.screen.settings.oss_licenses

data class OssLicenseEntry(
  val packageName: String,
  val paragraphs: List<String>,
)

internal fun normalizeOssLicenseEntries(entries: List<OssLicenseEntry>): List<OssLicenseEntry> {
  return entries.mapNotNull { entry ->
    val normalizedParagraphs = entry.paragraphs
      .map { it.trim() }
      .filter { it.isNotEmpty() }

    if (normalizedParagraphs.isEmpty()) {
      null
    } else {
      entry.copy(
        packageName = entry.packageName.trim(),
        paragraphs = normalizedParagraphs,
      )
    }
  }.sortedBy { it.packageName.lowercase() }
}
