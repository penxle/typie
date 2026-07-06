package co.typie.editor.render

import com.sun.jna.Pointer

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)
