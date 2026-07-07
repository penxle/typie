package co.typie.screen.editor.editor.overlay

import co.typie.editor.ffi.CharacterCounts
import co.typie.ext.comma

/** Collapsed widget label: document character count including whitespace. */
fun CharacterCounts.collapsedLabel(): String = "${docWithWhitespace.comma}자"

/**
 * Expanded widget detail rows appended below the always-visible header: the with-whitespace count
 * stays in the header and only these two rows are added. The full three-count breakdown lives in
 * the DocumentScreen document info, per the ticket ("글자 수 정보와 설정 entry는 DocumentScreen 쪽에 둔다").
 */
fun CharacterCounts.expandedRows(): List<Pair<String, String>> =
  listOf(
    "공백 미포함" to "${docWithoutWhitespace.comma}자",
    "공백/부호 미포함" to "${docWithoutWhitespaceAndPunctuation.comma}자",
  )
