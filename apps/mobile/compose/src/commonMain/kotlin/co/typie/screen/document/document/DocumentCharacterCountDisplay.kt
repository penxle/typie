package co.typie.screen.document.document

import co.typie.editor.ffi.CharacterCounts
import co.typie.ext.comma

internal fun documentCharacterCountSummary(
  counts: CharacterCounts?,
  fallbackWithWhitespace: Int,
): String = "${(counts?.docWithWhitespace ?: fallbackWithWhitespace).comma}자"

internal fun documentCharacterCountRows(
  counts: CharacterCounts?,
  fallbackWithWhitespace: Int,
): List<Pair<String, String>> {
  if (counts == null) {
    return listOf("공백 포함" to "${fallbackWithWhitespace.comma}자")
  }

  return listOf(
    "공백 포함" to
      formatDocumentCharacterCountRow(
        docCount = counts.docWithWhitespace,
        selectionCount = counts.selectionWithWhitespace,
      ),
    "공백 미포함" to
      formatDocumentCharacterCountRow(
        docCount = counts.docWithoutWhitespace,
        selectionCount = counts.selectionWithoutWhitespace,
      ),
    "공백/부호 미포함" to
      formatDocumentCharacterCountRow(
        docCount = counts.docWithoutWhitespaceAndPunctuation,
        selectionCount = counts.selectionWithoutWhitespaceAndPunctuation,
      ),
  )
}

private fun formatDocumentCharacterCountRow(docCount: Int, selectionCount: Int): String =
  if (selectionCount > 0) {
    "${selectionCount.comma}자 / ${docCount.comma}자"
  } else {
    "${docCount.comma}자"
  }
