@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.render

import kotlinx.cinterop.CPointer
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.IntVar
import kotlinx.cinterop.get
import kotlinx.cinterop.toCPointer

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray {
  if (count == 0) return IntArray(0)
  val p: CPointer<IntVar> = srcAddr.toCPointer() ?: return IntArray(0)
  return IntArray(count) { p[it] }
}
