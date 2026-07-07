package co.typie.screen.editor.editor.overlay

import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.debounce

/**
 * Collects a stream of editor document versions and invokes [fetch] once the stream settles for
 * [debounceMillis], so rapid typing does not trigger a character-count recomputation per keystroke.
 *
 * Extracted from the composable so the debounce policy is unit-testable with virtual time.
 */
@OptIn(FlowPreview::class)
suspend fun collectDebouncedCharacterCounts(
  versions: Flow<Long>,
  debounceMillis: Long,
  fetch: suspend () -> Unit,
) {
  versions.debounce(debounceMillis).collect { fetch() }
}
