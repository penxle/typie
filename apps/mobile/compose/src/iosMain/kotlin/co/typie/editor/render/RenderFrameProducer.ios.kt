@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.render

import kotlinx.cinterop.CPointer
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.IntVar
import kotlinx.cinterop.get
import kotlinx.cinterop.toCPointer
import org.jetbrains.skia.Pixmap

internal actual fun skiaPixelAddress(pixelMap: Any): Long = (pixelMap as Pixmap).addr.toLong()

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray {
  if (count == 0) return IntArray(0)
  val pointer: CPointer<IntVar> = srcAddr.toCPointer() ?: return IntArray(0)
  return IntArray(count) { pointer[it] }
}
