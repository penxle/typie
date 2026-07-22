package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertIs
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.io.Buffer

class FilePickerTest {
  @Test
  fun readableSelectionReturnsSelectedFiles() {
    val file = pickedFile("readable.txt")

    val result = aggregateSelectedFiles(listOf(Result.success(file)))

    val selected = assertIs<FilePickerResult.Selected>(result)
    assertEquals(listOf(file), selected.files)
    assertEquals(0, selected.unreadableCount)
  }

  @Test
  fun partialReadKeepsReadableFilesAndReportsUnreadableCount() {
    val first = pickedFile("first.txt")
    val second = pickedFile("second.txt")

    val result =
      aggregateSelectedFiles(
        listOf(
          Result.success(first),
          Result.failure(IllegalStateException("unreadable")),
          Result.success(second),
        )
      )

    val selected = assertIs<FilePickerResult.Selected>(result)
    assertEquals(listOf(first, second), selected.files)
    assertEquals(1, selected.unreadableCount)
  }

  @Test
  fun allReadsFailWithOriginalCause() {
    val firstFailure = IllegalStateException("first")

    val result =
      aggregateSelectedFiles(
        listOf(Result.failure(firstFailure), Result.failure(IllegalArgumentException("second")))
      )

    val failed = assertIs<FilePickerResult.Failed>(result)
    assertSame(firstFailure, failed.cause)
  }

  @Test
  fun pickedFileOpensContentLazilyAndReleasesItOnce() {
    var openCount = 0
    var releaseCount = 0
    val file =
      PickedFile(
        filename = "lazy.txt",
        mimeType = "text/plain",
        size = 4,
        previewModel = "preview",
        openSource = {
          openCount += 1
          Buffer()
        },
        release = { releaseCount += 1 },
      )

    assertEquals(0, openCount)
    assertEquals("preview", file.previewModel)

    file.openSource().close()
    file.close()
    file.close()

    assertEquals(1, openCount)
    assertEquals(1, releaseCount)
  }

  @Test
  fun svgMimeRecognitionNormalizesStandardMimeAndConservativeExtensionFallbacks() {
    assertEquals(
      "image/svg+xml",
      svgMimeTypeOrNull(filename = "drawing", mimeType = " IMAGE/SVG+XML ; charset=utf-8"),
    )

    for (mimeType in
      listOf(null, "application/octet-stream", "image/*", "text/xml", "application/xml")) {
      assertEquals(
        "image/svg+xml",
        svgMimeTypeOrNull(filename = " drawing.SvG ", mimeType = mimeType),
      )
    }

    assertEquals(null, svgMimeTypeOrNull(filename = "drawing.svg", mimeType = "image/png"))
    assertEquals(null, svgMimeTypeOrNull(filename = "drawing.xml", mimeType = "text/xml"))
  }

  @Test
  fun svgMimeUsesImageFilenameFallbackWithoutReplacingProvidedName() {
    assertEquals("image.svg", pickedFilename(originalFilename = null, mimeType = "image/svg+xml"))
    assertEquals(
      "drawing.svg",
      pickedFilename(originalFilename = " drawing.svg ", mimeType = "image/svg+xml"),
    )
  }

  @Test
  fun svgImageSizeUsesIntrinsicDimensionsAndRoundsPositiveFractions() {
    assertEquals(24 to 12, decodeSvgImageSize(svg("width=\"24\" height=\"12\"")))
    assertEquals(3 to 1, decodeSvgImageSize(svg("width=\"2.6\" height=\"0.4\"")))
  }

  @Test
  fun svgImageSizePreservesViewBoxAspectRatio() {
    val (width, height) = decodeSvgImageSize(svg("viewBox=\"0 0 48 32\""))

    assertTrue(width > 0)
    assertTrue(height > 0)
    assertEquals(width * 2, height * 3)
  }

  @Test
  fun svgImageSizeRejectsInvalidOrUnrepresentableDimensions() {
    for (bytes in
      listOf(
        "<svg".encodeToByteArray(),
        svg("width=\"0\" height=\"10\""),
        svg("width=\"-1\" height=\"10\""),
        svg("width=\"3000000000\" height=\"1\""),
      )) {
      assertFails { decodeSvgImageSize(bytes) }
    }
  }

  private fun pickedFile(filename: String): PickedFile =
    PickedFile(
      filename = filename,
      mimeType = "text/plain",
      size = filename.length.toLong(),
      previewModel = Unit,
      openSource = { Buffer() },
    )

  private fun svg(attributes: String): ByteArray =
    """<svg xmlns="http://www.w3.org/2000/svg" $attributes></svg>""".encodeToByteArray()
}
