package co.typie.editor

import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals

class FakeFfiEditorTest {

  @Test
  fun pageBackingSizes_can_differ_from_pageSizes() {
    val pageSizes = listOf(Size(width = 100f, height = 200f))
    val pageBackingSizes = listOf(Size(width = 120f, height = 240f))
    val editor =
      FakeFfiEditor(
        pageSizesProvider = { pageSizes },
        pageBackingSizesProvider = { pageBackingSizes },
      )

    assertEquals(pageSizes, editor.pageSizes())
    assertEquals(pageBackingSizes, editor.pageBackingSizes())
  }
}
