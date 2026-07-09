package co.typie.screen.document.document

import co.typie.editor.ffi.CharacterCounts
import kotlin.test.Test
import kotlin.test.assertEquals

class DocumentCharacterCountDisplayTest {
  @Test
  fun formatsDocumentCountsWithoutSelection() {
    val counts =
      CharacterCounts(
        docWithWhitespace = 1234,
        docWithoutWhitespace = 1000,
        docWithoutWhitespaceAndPunctuation = 900,
        selectionWithWhitespace = 0,
        selectionWithoutWhitespace = 0,
        selectionWithoutWhitespaceAndPunctuation = 0,
      )
    val rows = documentCharacterCountRows(counts = counts, fallbackWithWhitespace = 111)

    assertEquals("1,234자", documentCharacterCountSummary(counts, fallbackWithWhitespace = 111))
    assertEquals(listOf("공백 포함" to "1,234자", "공백 미포함" to "1,000자", "공백/부호 미포함" to "900자"), rows)
  }

  @Test
  fun formatsSelectionBeforeDocumentCount() {
    val counts =
      CharacterCounts(
        docWithWhitespace = 1234,
        docWithoutWhitespace = 1000,
        docWithoutWhitespaceAndPunctuation = 900,
        selectionWithWhitespace = 120,
        selectionWithoutWhitespace = 100,
        selectionWithoutWhitespaceAndPunctuation = 90,
      )
    val rows = documentCharacterCountRows(counts = counts, fallbackWithWhitespace = 111)

    assertEquals("1,234자", documentCharacterCountSummary(counts, fallbackWithWhitespace = 111))
    assertEquals(
      listOf("공백 포함" to "120자 / 1,234자", "공백 미포함" to "100자 / 1,000자", "공백/부호 미포함" to "90자 / 900자"),
      rows,
    )
  }

  @Test
  fun fallsBackToServerWhitespaceCountWhenEditorCountsAreUnavailable() {
    val rows = documentCharacterCountRows(counts = null, fallbackWithWhitespace = 1234)

    assertEquals(
      "1,234자",
      documentCharacterCountSummary(counts = null, fallbackWithWhitespace = 1234),
    )
    assertEquals(listOf("공백 포함" to "1,234자"), rows)
  }
}
