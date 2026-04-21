@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.render

import kotlinx.cinterop.ExperimentalForeignApi
import swiftPMImport.co.typie.compose.RenderBuffer as SwiftRenderBuffer

internal actual object RenderBuffer {
  actual fun allocate(width: Int, height: Int): Long = SwiftRenderBuffer.allocate(width, height)

  actual fun free(handle: Long) = SwiftRenderBuffer.free(handle)

  actual fun resize(handle: Long, width: Int, height: Int) =
    SwiftRenderBuffer.resize(handle, width, height)

  actual fun beginRead(handle: Long): Boolean = SwiftRenderBuffer.beginRead(handle)

  actual fun endRead(handle: Long) = SwiftRenderBuffer.endRead(handle)

  actual fun getDataPointer(handle: Long): Long = SwiftRenderBuffer.dataPointer(handle)

  actual fun getPixelWidth(handle: Long): Int = SwiftRenderBuffer.width(handle)

  actual fun getPixelHeight(handle: Long): Int = SwiftRenderBuffer.height(handle)
}
