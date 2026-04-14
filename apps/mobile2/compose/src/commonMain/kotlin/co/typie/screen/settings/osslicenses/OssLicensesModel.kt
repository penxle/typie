package co.typie.screen.settings.osslicenses

import kotlinx.serialization.Serializable

data class OssLicenseEntry(val packageName: String, val paragraphs: List<String>)

internal fun AboutLibraries.toEntry(): List<OssLicenseEntry> {
  return libraries
    .mapNotNull { library ->
      val packageName = library.uniqueId.trim()
      if (packageName.isEmpty()) return@mapNotNull null

      val paragraphs = library.licenses.flatMap { id -> licenses[id]?.toParagraphs().orEmpty() }
      if (paragraphs.isEmpty()) return@mapNotNull null

      OssLicenseEntry(packageName = packageName, paragraphs = paragraphs)
    }
    .sortedBy { it.packageName.lowercase() }
}

private fun AboutLibrariesLicense.toParagraphs(): List<String> {
  val source =
    content?.trim().takeIf { !it.isNullOrEmpty() }
      ?: name?.trim().takeIf { !it.isNullOrEmpty() }
      ?: return emptyList()

  return source.split(Regex("""\n\s*\n""")).map { it.trim() }.filter { it.isNotEmpty() }
}

@Serializable
internal data class AboutLibraries(
  val libraries: List<AboutLibrariesLibrary> = emptyList(),
  val licenses: Map<String, AboutLibrariesLicense> = emptyMap(),
)

@Serializable
internal data class AboutLibrariesLibrary(
  val uniqueId: String = "",
  val licenses: List<String> = emptyList(),
)

@Serializable
internal data class AboutLibrariesLicense(val name: String? = null, val content: String? = null)
