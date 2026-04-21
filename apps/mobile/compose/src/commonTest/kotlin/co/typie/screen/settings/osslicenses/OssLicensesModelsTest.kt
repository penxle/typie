package co.typie.screen.settings.osslicenses

import kotlin.test.Test
import kotlin.test.assertEquals

class OssLicensesModelsTest {
  @Test
  fun `toEntry trims text and sorts packages by name`() {
    val entries =
      AboutLibraries(
          libraries =
            listOf(
              AboutLibrariesLibrary(uniqueId = "zeta", licenses = listOf("apache")),
              AboutLibrariesLibrary(uniqueId = "Alpha", licenses = listOf("mit")),
            ),
          licenses =
            mapOf(
              "apache" to AboutLibrariesLicense(content = " Apache License 2.0 \n\n Copyright"),
              "mit" to AboutLibrariesLicense(name = " MIT License "),
            ),
        )
        .toEntry()

    assertEquals(listOf("Alpha", "zeta"), entries.map { it.packageName })
    assertEquals(listOf("MIT License"), entries.first().paragraphs)
    assertEquals(listOf("Apache License 2.0", "Copyright"), entries.last().paragraphs)
  }

  @Test
  fun `toEntry drops packages with no usable license text`() {
    val entries =
      AboutLibraries(
          libraries =
            listOf(
              AboutLibrariesLibrary(uniqueId = "empty", licenses = listOf("blank")),
              AboutLibrariesLibrary(uniqueId = "valid", licenses = listOf("bsd")),
            ),
          licenses =
            mapOf(
              "blank" to AboutLibrariesLicense(content = "   "),
              "bsd" to AboutLibrariesLicense(name = "BSD-3-Clause"),
            ),
        )
        .toEntry()

    assertEquals(listOf("valid"), entries.map { it.packageName })
  }
}
