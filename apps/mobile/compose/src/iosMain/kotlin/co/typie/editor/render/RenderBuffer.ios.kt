@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.editor.render

import swiftPMImport.co.typie.compose.RenderBuffer as SwiftRenderBuffer

internal actual object RenderBuffer {
  actual fun allocate(): Long = SwiftRenderBuffer.allocate()

  actual fun free(handle: Long): Unit = SwiftRenderBuffer.free(handle)

  actual fun beginRead(handle: Long): Boolean = SwiftRenderBuffer.beginRead(handle)

  actual fun endRead(handle: Long): Unit = SwiftRenderBuffer.endRead(handle)

  actual fun getPixelWidth(handle: Long): Int = SwiftRenderBuffer.width(handle)

  actual fun getPixelHeight(handle: Long): Int = SwiftRenderBuffer.height(handle)

  actual fun getPinnedEditorRevision(handle: Long): Long =
    SwiftRenderBuffer.pinnedEditorRevision(handle)

  actual fun getPinnedFrameKey(handle: Long): Long = SwiftRenderBuffer.pinnedFrameKey(handle)

  actual fun getPinnedTileCount(handle: Long): Int = SwiftRenderBuffer.pinnedTileCount(handle)

  actual fun getPinnedTileBounds(handle: Long, index: Int): Long =
    SwiftRenderBuffer.pinnedTileBounds(handle, index)

  actual fun getPinnedTilePixels(handle: Long, index: Int): Long =
    SwiftRenderBuffer.pinnedTilePixels(handle, index)

  actual fun getPinnedTileVersion(handle: Long, index: Int): Long =
    SwiftRenderBuffer.pinnedTileVersion(handle, index)

  actual fun readPinnedTileInto(handle: Long, index: Int, dstAddr: Long, dstLen: Long): Boolean =
    SwiftRenderBuffer.readPinnedTileInto(handle, index, dstAddr, dstLen)
}
