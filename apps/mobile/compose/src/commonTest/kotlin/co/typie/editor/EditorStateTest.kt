package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertEquals

class EditorStateTest {
  @Test
  fun initial_has_version_zero_and_null_fields() {
    val s = EditorState.Initial
    assertEquals(0L, s.version)
    assertEquals(0L, s.documentRevision)
    assertEquals(null, s.cursor)
    assertEquals(null, s.placeholder)
    assertEquals(null, s.selection)
    assertEquals(emptyList(), s.pageSizes)
    assertEquals(emptyList(), s.externalElements)
    assertEquals(null, s.rootAttrs)
    assertEquals(null, s.ime)
  }
}
