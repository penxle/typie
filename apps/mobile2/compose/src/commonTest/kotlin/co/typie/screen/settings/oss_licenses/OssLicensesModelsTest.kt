package co.typie.screen.settings.oss_licenses

import kotlin.test.Test
import kotlin.test.assertEquals

class OssLicensesModelsTest {
  @Test
  fun `normalizeOssLicenseEntries trims text and sorts packages by name`() {
    val entries = normalizeOssLicenseEntries(
      listOf(
        OssLicenseEntry(
          packageName = "zeta",
          paragraphs = listOf(" Apache License 2.0 ", "", " Copyright"),
        ),
        OssLicenseEntry(
          packageName = "Alpha",
          paragraphs = listOf(" MIT License "),
        ),
      ),
    )

    assertEquals(listOf("Alpha", "zeta"), entries.map { it.packageName })
    assertEquals(listOf("MIT License"), entries.first().paragraphs)
    assertEquals(listOf("Apache License 2.0", "Copyright"), entries.last().paragraphs)
  }

  @Test
  fun `normalizeOssLicenseEntries drops packages with no usable license text`() {
    val entries = normalizeOssLicenseEntries(
      listOf(
        OssLicenseEntry(
          packageName = "empty",
          paragraphs = listOf("", "   "),
        ),
        OssLicenseEntry(
          packageName = "valid",
          paragraphs = listOf("BSD-3-Clause"),
        ),
      ),
    )

    assertEquals(listOf("valid"), entries.map { it.packageName })
  }
}
