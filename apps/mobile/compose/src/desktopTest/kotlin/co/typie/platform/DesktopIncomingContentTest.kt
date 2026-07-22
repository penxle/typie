package co.typie.platform

import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.Transferable
import java.awt.image.BufferedImage
import java.io.File
import java.nio.file.Files
import javax.imageio.ImageIO
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest
import kotlinx.io.readByteArray

class DesktopIncomingContentTest {
  @Test
  fun nonemptyHtmlDoesNotRequestAttachmentFlavors() = runTest {
    var attachmentReads = 0
    val transferable =
      FakeTransferable(
        values =
          linkedMapOf(
            DataFlavor.allHtmlFlavor to "<p>rich</p>",
            DataFlavor.stringFlavor to "rich",
            DataFlavor.imageFlavor to BufferedImage(2, 2, BufferedImage.TYPE_INT_ARGB),
          ),
        onRead = { flavor ->
          if (flavor == DataFlavor.imageFlavor || flavor == DataFlavor.javaFileListFlavor) {
            attachmentReads += 1
          }
        },
      )

    val candidates = transferable.readIncomingContentCandidates()

    assertEquals(0, attachmentReads)
    assertEquals("<p>rich</p>", candidates?.html)
    candidates?.close()
  }

  @Test
  fun orderedFileListWinsOverDuplicateImageFlavor() {
    val directory = Files.createTempDirectory("typie-clipboard-").toFile()
    try {
      val imageFile = File(directory, "first.png")
      ImageIO.write(BufferedImage(2, 3, BufferedImage.TYPE_INT_ARGB), "png", imageFile)
      val textFile = File(directory, "second.txt").apply { writeText("file") }
      val transferable =
        FakeTransferable(
          linkedMapOf(
            DataFlavor.javaFileListFlavor to listOf(imageFile, textFile),
            DataFlavor.imageFlavor to BufferedImage(9, 9, BufferedImage.TYPE_INT_ARGB),
          )
        )

      val items = transferable.readAttachmentItems().items

      assertEquals(
        listOf(IncomingContentItem.Kind.Image, IncomingContentItem.Kind.File),
        items.map(IncomingContentItem::kind),
      )
      assertEquals(listOf("first.png", "second.txt"), items.map { it.file.filename })
      assertEquals(2, items.first().file.imageWidth)
      assertEquals(3, items.first().file.imageHeight)
      items.forEach { it.file.close() }
    } finally {
      directory.deleteRecursively()
    }
  }

  @Test
  fun orderedFileListMaterializesSvgAsOriginalImageWithoutDuplicateImageFlavor() {
    val directory = Files.createTempDirectory("typie-svg-clipboard-").toFile()
    try {
      val svgBytes =
        """<svg xmlns="http://www.w3.org/2000/svg" width="12" height="8"></svg>"""
          .encodeToByteArray()
      val imageFile = File(directory, "first.svg").apply { writeBytes(svgBytes) }
      val textFile = File(directory, "second.txt").apply { writeText("file") }
      val transferable =
        FakeTransferable(
          linkedMapOf(
            DataFlavor.javaFileListFlavor to listOf(imageFile, textFile),
            DataFlavor.imageFlavor to BufferedImage(9, 9, BufferedImage.TYPE_INT_ARGB),
          )
        )

      val items = transferable.readAttachmentItems().items

      assertEquals(
        listOf(IncomingContentItem.Kind.Image, IncomingContentItem.Kind.File),
        items.map(IncomingContentItem::kind),
      )
      assertEquals(listOf("first.svg", "second.txt"), items.map { it.file.filename })
      assertEquals("image/svg+xml", items.first().file.mimeType)
      assertEquals(12, items.first().file.imageWidth)
      assertEquals(8, items.first().file.imageHeight)
      assertContentEquals(svgBytes, items.first().file.openSource().use { it.readByteArray() })
      items.forEach { it.file.close() }
    } finally {
      directory.deleteRecursively()
    }
  }

  @Test
  fun imageFlavorMaterializesOnePngCandidate() {
    val image = BufferedImage(4, 5, BufferedImage.TYPE_INT_ARGB)

    val items = FakeTransferable(mapOf(DataFlavor.imageFlavor to image)).readAttachmentItems().items

    assertEquals(1, items.size)
    val file = items.single().file
    assertEquals(IncomingContentItem.Kind.Image, items.single().kind)
    assertEquals("image/png", file.mimeType)
    assertEquals(4, file.imageWidth)
    assertEquals(5, file.imageHeight)
    assertTrue(requireNotNull(file.size) > 0)
    file.close()
  }

  @Test
  fun unreadableAdvertisedImageIsReported() {
    val loaded = FakeTransferable(mapOf(DataFlavor.imageFlavor to Unit)).readAttachmentItems()

    assertEquals(emptyList(), loaded.items)
    assertEquals(1, loaded.unreadableItemCount)
  }

  @Test
  fun unreadableFileListItemDoesNotDiscardReadableSibling() {
    val file = Files.createTempFile("typie-clipboard-", ".txt").toFile()
    try {
      val loaded =
        FakeTransferable(mapOf(DataFlavor.javaFileListFlavor to listOf(file, "not-a-file")))
          .readAttachmentItems()

      assertEquals(listOf(file.name), loaded.items.map { it.file.filename })
      assertEquals(1, loaded.unreadableItemCount)
      loaded.items.forEach { it.file.close() }
    } finally {
      file.delete()
    }
  }

  @Test
  fun htmlAndStringFlavorsRemainAvailable() {
    val transferable =
      FakeTransferable(
        mapOf(DataFlavor.allHtmlFlavor to "<p>rich</p>", DataFlavor.stringFlavor to "rich")
      )

    assertEquals("<p>rich</p>", transferable.readHtml())
    assertEquals("rich", transferable.readString(DataFlavor.stringFlavor))
    assertNotNull(transferable.transferDataFlavors)
  }

  @Test
  fun fragmentHtmlFlavorIsAcceptedWhenAllHtmlIsAbsent() {
    val transferable =
      FakeTransferable(mapOf(DataFlavor.fragmentHtmlFlavor to "<strong>fragment</strong>"))

    assertEquals("<strong>fragment</strong>", transferable.readHtml())
  }

  private class FakeTransferable(
    private val values: Map<DataFlavor, Any>,
    private val onRead: (DataFlavor) -> Unit = {},
  ) : Transferable {
    override fun getTransferDataFlavors(): Array<DataFlavor> = values.keys.toTypedArray()

    override fun isDataFlavorSupported(flavor: DataFlavor): Boolean = values.containsKey(flavor)

    override fun getTransferData(flavor: DataFlavor): Any {
      onRead(flavor)
      return values[flavor] ?: error("Unsupported flavor: $flavor")
    }
  }
}
