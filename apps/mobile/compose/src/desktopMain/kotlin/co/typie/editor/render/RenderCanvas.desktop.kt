package co.typie.editor.render

import com.sun.jna.Pointer

internal actual fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int) {
  Pointer(srcAddr).read(0, dst, 0, length)
}

internal actual fun copyNativeBytesRange(srcAddr: Long, dst: ByteArray, offset: Int, length: Int) {
  Pointer(srcAddr).read(offset.toLong(), dst, offset, length)
}

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)
