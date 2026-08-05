package co.typie.editor.render

import com.sun.jna.Pointer
import org.jetbrains.skia.Pixmap

internal actual fun skiaPixelAddress(pixelMap: Any): Long = (pixelMap as Pixmap).addr

internal actual fun readNativeInts(srcAddr: Long, count: Int): IntArray =
  Pointer(srcAddr).getIntArray(0, count)
