package co.typie.editor.sync

import kotlin.test.Test
import kotlin.test.assertContentEquals

class SyncBytesTest {
  @Test
  fun highValueBytesRoundTrip() {
    assertContentEquals(byteArrayOf(0, 127, -128, -1), listOf(0, 127, 128, 255).toChangesetBytes())
  }

  @Test
  fun emptyListYieldsEmptyArray() {
    assertContentEquals(ByteArray(0), emptyList<Int>().toChangesetBytes())
  }
}
