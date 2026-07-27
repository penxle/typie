package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ShareFilenameTest {
  @Test
  fun `keeps an ordinary filename`() {
    assertEquals("내 문서 - 부제.pdf", sanitizeShareFilename("내 문서 - 부제.pdf"))
  }

  @Test
  fun `replaces path separators`() {
    assertEquals("a_b_c.pdf", sanitizeShareFilename("a/b\\c.pdf"))
  }

  @Test
  fun `replaces control characters`() {
    assertEquals("a_b.pdf", sanitizeShareFilename("a\tb.pdf"))
    assertEquals("a_b.pdf", sanitizeShareFilename("a\nb.pdf"))
  }

  @Test
  fun `trims surrounding whitespace`() {
    assertEquals("doc.pdf", sanitizeShareFilename("  doc.pdf  "))
  }

  @Test
  fun `falls back when nothing usable is left`() {
    assertEquals("file", sanitizeShareFilename("   "))
    assertEquals("file", sanitizeShareFilename(""))
    assertEquals("file", sanitizeShareFilename("."))
    assertEquals("file", sanitizeShareFilename(".."))
  }

  @Test
  fun `keeps a name that is exactly at the byte limit`() {
    val name = "a".repeat(251) + ".pdf"

    assertEquals(255, name.encodeToByteArray().size)
    assertEquals(name, sanitizeShareFilename(name))
  }

  @Test
  fun `truncates a long name by bytes while keeping the extension`() {
    val result = sanitizeShareFilename("가".repeat(300) + ".pdf")

    assertEquals("가".repeat(83) + ".pdf", result)
    assertTrue(result.encodeToByteArray().size <= 255)
  }

  @Test
  fun `truncates a long name without an extension`() {
    val result = sanitizeShareFilename("가".repeat(300))

    assertEquals("가".repeat(85), result)
    assertEquals(255, result.encodeToByteArray().size)
  }

  @Test
  fun `never splits a surrogate pair`() {
    val result = sanitizeShareFilename("😀".repeat(200) + ".pdf")

    assertEquals("😀".repeat(62) + ".pdf", result)
    assertEquals(result, result.encodeToByteArray().decodeToString())
  }

  @Test
  fun `drops whitespace left at the truncation point`() {
    val result = sanitizeShareFilename("가".repeat(83) + " 가".repeat(20) + ".pdf")

    assertEquals("가".repeat(83) + ".pdf", result)
  }

  @Test
  fun `truncates a name whose extension alone exceeds the limit`() {
    val result = sanitizeShareFilename("file." + "가".repeat(300))

    assertEquals("file." + "가".repeat(83), result)
    assertTrue(result.encodeToByteArray().size <= 255)
  }
}
