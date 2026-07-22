@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import kotlinx.io.readByteArray
import platform.CoreGraphics.CGSizeMake
import platform.Foundation.NSData
import platform.Foundation.NSFileManager
import platform.Foundation.NSTemporaryDirectory
import platform.Foundation.NSURL
import platform.Foundation.NSUUID
import platform.Foundation.create
import platform.Foundation.writeToFile
import platform.UIKit.UIGraphicsBeginImageContextWithOptions
import platform.UIKit.UIGraphicsEndImageContext
import platform.UIKit.UIGraphicsGetImageFromCurrentImageContext
import platform.UIKit.UIImagePNGRepresentation
import platform.UniformTypeIdentifiers.UTTypeSVG

class IOSFilePickerTest {
  @Test
  fun ownedSvgFilePreservesBytesDimensionsAndDeletesOnClose() {
    val bytes = svgBytes(width = 18, height = 12)
    val url = temporaryFile("owned.svg", bytes)
    try {
      val file = url.toPickedImage(filename = "owned.svg", mimeType = "image/svg+xml")

      assertEquals(18, file.imageWidth)
      assertEquals(12, file.imageHeight)
      assertContentEquals(bytes, file.openSource().use { it.readByteArray() })
      assertTrue(NSFileManager.defaultManager.fileExistsAtPath(requireNotNull(url.path)))

      file.close()

      assertFalse(NSFileManager.defaultManager.fileExistsAtPath(requireNotNull(url.path)))
    } finally {
      NSFileManager.defaultManager.removeItemAtURL(url, error = null)
    }
  }

  @Test
  fun rawSvgDataPreservesBytesAndDeletesItsOwnedFileOnClose() {
    val bytes = svgBytes(width = 15, height = 10)

    val imported =
      assertNotNull(readDirectPasteboardAttachment(mapOf(UTTypeSVG.identifier to bytes.toNSData())))
    assertEquals(IncomingContentItem.Kind.Image, imported.kind)
    val file = imported.file
    val url = file.previewModel as NSURL
    try {
      assertEquals("image.svg", file.filename)
      assertEquals("image/svg+xml", file.mimeType)
      assertEquals(15, file.imageWidth)
      assertEquals(10, file.imageHeight)
      assertContentEquals(bytes, file.openSource().use { it.readByteArray() })
      assertTrue(NSFileManager.defaultManager.fileExistsAtPath(requireNotNull(url.path)))

      file.close()

      assertFalse(NSFileManager.defaultManager.fileExistsAtPath(requireNotNull(url.path)))
    } finally {
      file.close()
    }
  }

  @Test
  fun malformedRawSvgDataRemovesItsTemporaryFile() {
    val existingFiles = temporaryFilesEndingWith("image.svg")

    assertFails {
      readDirectPasteboardAttachment(
        mapOf(UTTypeSVG.identifier to "<svg".encodeToByteArray().toNSData())
      )
    }

    assertEquals(existingFiles, temporaryFilesEndingWith("image.svg"))
  }

  @Test
  fun providerSelectionPrefersRawSvgAndPreservesOrderWithinRank() {
    val raster =
      PasteboardProviderRepresentation(
        identifier = "public.png",
        kind = IncomingContentItem.Kind.Image,
        filename = "image.png",
        mimeType = "image/png",
      )
    val secondRaster = raster.copy(identifier = "public.jpeg", filename = "image.jpg")
    val svg =
      PasteboardProviderRepresentation(
        identifier = UTTypeSVG.identifier,
        kind = IncomingContentItem.Kind.Image,
        filename = "image.svg",
        mimeType = "image/svg+xml",
      )
    val file =
      PasteboardProviderRepresentation(
        identifier = "public.data",
        kind = IncomingContentItem.Kind.File,
        filename = "file",
        mimeType = "application/octet-stream",
      )

    assertSame(svg, selectPasteboardProviderRepresentation(listOf(raster, svg, file)))
    assertSame(raster, selectPasteboardProviderRepresentation(listOf(raster, secondRaster)))
  }

  @Test
  fun directPasteboardItemPrefersRawSvgOverRasterFileUrlWithoutFallbackOnFailure() {
    val rasterUrl = temporaryPngFile()
    try {
      val svgBytes = svgBytes(width = 21, height = 14)
      val item =
        mapOf<Any, Any>("public.file-url" to rasterUrl, UTTypeSVG.identifier to svgBytes.toNSData())

      val imported = assertNotNull(readDirectPasteboardAttachment(item))
      try {
        assertEquals(IncomingContentItem.Kind.Image, imported.kind)
        assertEquals("image/svg+xml", imported.file.mimeType)
        assertContentEquals(svgBytes, imported.file.openSource().use { it.readByteArray() })
      } finally {
        imported.file.close()
      }

      assertFails {
        readDirectPasteboardAttachment(
          mapOf<Any, Any>(
            "public.file-url" to rasterUrl,
            UTTypeSVG.identifier to "<svg".encodeToByteArray().toNSData(),
          )
        )
      }
    } finally {
      NSFileManager.defaultManager.removeItemAtURL(rasterUrl, error = null)
    }
  }

  private fun svgBytes(width: Int, height: Int): ByteArray =
    """<svg xmlns="http://www.w3.org/2000/svg" width="$width" height="$height"></svg>"""
      .encodeToByteArray()

  private fun temporaryFile(filename: String, bytes: ByteArray): NSURL {
    val path = "${NSTemporaryDirectory()}${NSUUID().UUIDString}-$filename"
    check(bytes.toNSData().writeToFile(path, atomically = true))
    return NSURL.fileURLWithPath(path)
  }

  private fun temporaryPngFile(): NSURL {
    UIGraphicsBeginImageContextWithOptions(CGSizeMake(2.0, 3.0), false, 1.0)
    val image =
      try {
        requireNotNull(UIGraphicsGetImageFromCurrentImageContext())
      } finally {
        UIGraphicsEndImageContext()
      }
    val data = requireNotNull(UIImagePNGRepresentation(image))
    val path = "${NSTemporaryDirectory()}${NSUUID().UUIDString}-image.png"
    check(data.writeToFile(path, atomically = true))
    return NSURL.fileURLWithPath(path)
  }

  private fun temporaryFilesEndingWith(suffix: String): Set<String> =
    NSFileManager.defaultManager
      .contentsOfDirectoryAtPath(NSTemporaryDirectory(), error = null)
      ?.filterIsInstance<String>()
      ?.filter { it.endsWith(suffix) }
      ?.toSet()
      .orEmpty()

  private fun ByteArray.toNSData(): NSData = usePinned { pinned ->
    NSData.create(bytes = pinned.addressOf(0), length = size.toULong())
  }
}
