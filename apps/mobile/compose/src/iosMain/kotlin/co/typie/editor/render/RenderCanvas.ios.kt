@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.render

import kotlinx.cinterop.ByteVar
import kotlinx.cinterop.CPointer
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.toCPointer
import kotlinx.cinterop.usePinned
import platform.posix.memcpy

internal actual fun copyNativeBytes(srcAddr: Long, dst: ByteArray, length: Int) {
  if (length == 0) return
  val srcPtr: CPointer<ByteVar> = srcAddr.toCPointer() ?: return
  dst.usePinned { pinned -> memcpy(pinned.addressOf(0), srcPtr, length.toULong()) }
}
