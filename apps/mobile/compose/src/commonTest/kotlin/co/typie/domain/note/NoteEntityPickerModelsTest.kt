package co.typie.domain.note

import co.typie.graphql.QueryState
import kotlin.test.Test
import kotlin.test.assertEquals

class NoteEntityPickerModelsTest {
  @Test
  fun `resolveRecentNotePickerEntities returns placeholders while recent query loads`() {
    assertEquals(
      listOf("placeholder-1", "placeholder-2"),
      resolveRecentNotePickerEntities(
        inputKeyword = "",
        recentQueryState = QueryState.Loading,
        settledEntities = listOf("real"),
        placeholderEntities = listOf("placeholder-1", "placeholder-2"),
      ),
    )
  }

  @Test
  fun `resolveRecentNotePickerEntities returns settled entities after recent query succeeds`() {
    assertEquals(
      listOf("real-1", "real-2"),
      resolveRecentNotePickerEntities(
        inputKeyword = "",
        recentQueryState = QueryState.Success(Unit),
        settledEntities = listOf("real-1", "real-2"),
        placeholderEntities = listOf("placeholder"),
      ),
    )
  }

  @Test
  fun `resolveRecentNotePickerEntities hides recent placeholders during search and error states`() {
    assertEquals(
      emptyList<String>(),
      resolveRecentNotePickerEntities(
        inputKeyword = "doc",
        recentQueryState = QueryState.Loading,
        settledEntities = listOf("real"),
        placeholderEntities = listOf("placeholder"),
      ),
    )
    assertEquals(
      emptyList<String>(),
      resolveRecentNotePickerEntities(
        inputKeyword = "",
        recentQueryState = QueryState.Error(Exception("boom")),
        settledEntities = listOf("real"),
        placeholderEntities = listOf("placeholder"),
      ),
    )
  }
}
