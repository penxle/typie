package co.typie.editor.render

import com.sun.jna.Pointer

internal actual fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int) {
  Pointer(srcAddr).read(0, dst, 0, length)
}
