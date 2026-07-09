package co.typie.editor.sync

import kotlin.test.Test
import kotlin.test.assertContentEquals

class ConcatBytesTest {
  @Test
  fun concatenatesInOrder() {
    assertContentEquals(
      byteArrayOf(1, 2, 3, 4),
      listOf(byteArrayOf(1, 2), byteArrayOf(), byteArrayOf(3, 4)).concatChangesets(),
    )
  }

  @Test
  fun emptyListYieldsEmptyArray() {
    assertContentEquals(ByteArray(0), emptyList<ByteArray>().concatChangesets())
  }

  @Test
  fun lengthPrefixedBlobsMatchFfiFormat() {
    val encoded = encodeLengthPrefixedBlobs(listOf(byteArrayOf(9, 8), byteArrayOf(7)))
    assertContentEquals(byteArrayOf(2, 0, 0, 0, 2, 0, 0, 0, 9, 8, 1, 0, 0, 0, 7), encoded)
  }

  @Test
  fun lengthPrefixedEmptyListIsBareCount() {
    assertContentEquals(byteArrayOf(0, 0, 0, 0), encodeLengthPrefixedBlobs(emptyList()))
  }
}
