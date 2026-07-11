package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertSame
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

  private fun pickedFile(filename: String): PickedFile =
    PickedFile(
      filename = filename,
      mimeType = "text/plain",
      size = filename.length.toLong(),
      previewModel = Unit,
      openSource = { Buffer() },
    )
}
