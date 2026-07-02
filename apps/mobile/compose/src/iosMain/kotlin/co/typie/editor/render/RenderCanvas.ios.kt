@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.render

import kotlinx.cinterop.ByteVar
import kotlinx.cinterop.CPointer
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.IntVar
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.get
import kotlinx.cinterop.toCPointer
import kotlinx.cinterop.usePinned
import platform.posix.memcpy

internal actual fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int) {
  if (length == 0) return
  val srcPtr: CPointer<ByteVar> = srcAddr.toCPointer() ?: return
  dst.usePinned { pinned -> memcpy(pinned.addressOf(0), srcPtr, length.toULong()) }
}

internal actual fun copyNativeBytesRange(srcAddr: Long, dst: ByteArray, offset: Int, length: Int) {
  if (length == 0) return
  val srcPtr: CPointer<ByteVar> = (srcAddr + offset.toLong()).toCPointer() ?: return
  dst.usePinned { pinned -> memcpy(pinned.addressOf(offset), srcPtr, length.toULong()) }
}

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray {
  if (count == 0) return IntArray(0)
  val p: CPointer<IntVar> = srcAddr.toCPointer() ?: return IntArray(0)
  return IntArray(count) { p[it] }
}
