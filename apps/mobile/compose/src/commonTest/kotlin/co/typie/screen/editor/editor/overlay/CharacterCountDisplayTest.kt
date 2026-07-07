package co.typie.screen.editor.editor.overlay

import co.typie.editor.ffi.CharacterCounts
import kotlin.test.Test
import kotlin.test.assertEquals

class CharacterCountDisplayTest {
  private val counts =
    CharacterCounts(
      docWithWhitespace = 1234,
      docWithoutWhitespace = 1000,
      docWithoutWhitespaceAndPunctuation = 900,
      selectionWithWhitespace = 0,
      selectionWithoutWhitespace = 0,
      selectionWithoutWhitespaceAndPunctuation = 0,
    )

  @Test
  fun collapsedShowsWithWhitespaceOnly() {
    assertEquals("1,234자", counts.collapsedLabel())
  }

  @Test
  fun expandedShowsDetailRowsBelowHeader() {
    // The with-whitespace count stays in the always-visible header and expanding appends only the
    // two detail rows; the full breakdown lives in the DocumentScreen document info.
    val rows = counts.expandedRows()

    assertEquals(2, rows.size)
    assertEquals("공백 미포함" to "1,000자", rows[0])
    assertEquals("공백/부호 미포함" to "900자", rows[1])
  }

  @Test
  fun formatsZeroWithComma() {
    val zero =
      CharacterCounts(
        docWithWhitespace = 0,
        docWithoutWhitespace = 0,
        docWithoutWhitespaceAndPunctuation = 0,
        selectionWithWhitespace = 0,
        selectionWithoutWhitespace = 0,
        selectionWithoutWhitespaceAndPunctuation = 0,
      )

    assertEquals("0자", zero.collapsedLabel())
  }
}
