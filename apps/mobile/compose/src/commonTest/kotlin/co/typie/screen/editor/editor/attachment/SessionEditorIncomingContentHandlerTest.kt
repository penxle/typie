package co.typie.screen.editor.editor.attachment

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentCandidates
import co.typie.platform.IncomingContentItem
import co.typie.platform.IncomingContentMode
import co.typie.platform.PickedFile
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import kotlinx.io.Buffer

class SessionEditorIncomingContentHandlerTest {
  @Test
  fun htmlWinsWithoutAlsoImportingNativeAttachmentCandidates() = runTest {
    var released = false
    var importCalls = 0
    val fake = FakeFfiEditor()
    val editor = Editor(fake, this)
    val session = createTestDocumentEditingSession(editor, this)
    val handler =
      SessionEditorIncomingContentHandler(
        importer =
          EditorAttachmentImporter { _, _, _, _ ->
            importCalls += 1
            true
          },
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        isSessionCurrent = { it === session },
        onAttachmentError = {},
      )

    val handled =
      handler.handleCandidates(
        session = session,
        candidates =
          IncomingContentCandidates(
            html = "<p>before<img src=\"https://example.com/image.png\">after</p>",
            text = "before after",
            items = listOf(imageItem { released = true }),
          ),
        mode = IncomingContentMode.Rich,
      )

    assertTrue(handled)
    assertEquals(0, importCalls)
    assertTrue(released)
    assertTrue(
      fake.enqueued.contains(
        Message.Clipboard(
          ClipboardOp.Paste(
            html = "<p>before<img src=\"https://example.com/image.png\">after</p>",
            text = "before after",
          )
        )
      )
    )
  }

  @Test
  fun attachmentOnlyContentTransfersOwnershipAtCurrentSelectionAndReturnsImporterBoolean() =
    runTest {
      for (importerResult in listOf(false, true)) {
        var released = false
        var importedItems = emptyList<IncomingContentItem>()
        lateinit var completeImport: (Int) -> Unit
        val editor = Editor(FakeFfiEditor(), this)
        val session = createTestDocumentEditingSession(editor, this)
        val handler =
          SessionEditorIncomingContentHandler(
            importer =
              EditorAttachmentImporter { importedSession, items, destination, onCompleted ->
                assertTrue(importedSession === session)
                assertEquals(EditorAttachmentDestination.CurrentSelection, destination)
                importedItems = items
                completeImport = onCompleted
                items.forEach { it.file.close() }
                importerResult
              },
            bringIntoViewRequests = EditorBringIntoViewRequests(),
            isSessionCurrent = { it === session },
            onAttachmentError = {},
          )

        val handled =
          handler.handleCandidates(
            session,
            IncomingContentCandidates(items = listOf(imageItem { released = true })),
          )

        assertEquals(importerResult, handled)
        assertEquals(1, importedItems.size)
        assertTrue(released)
        completeImport(if (importerResult) 1 else 0)
      }
    }

  @Test
  fun imageOnlyTotalFailureUsesImageErrorCopy() = runTest {
    assertCompletionError(
      items = mutableListOf(imageItem(), imageItem()),
      accepted = false,
      importedCount = 0,
      expectedError = "이미지를 삽입하지 못했어요.",
    )
  }

  @Test
  fun imageOnlyPartialFailureUsesPartialImageErrorCopy() = runTest {
    assertCompletionError(
      items = mutableListOf(imageItem(), imageItem()),
      accepted = true,
      importedCount = 1,
      expectedError = "일부 이미지를 삽입하지 못했어요.",
    )
  }

  @Test
  fun fileOnlyTotalFailureUsesFileErrorCopy() = runTest {
    assertCompletionError(
      items = mutableListOf(fileItem(), fileItem()),
      accepted = true,
      importedCount = 0,
      expectedError = "파일을 삽입하지 못했어요.",
    )
  }

  @Test
  fun fileOnlyPartialFailureUsesPartialFileErrorCopy() = runTest {
    assertCompletionError(
      items = mutableListOf(fileItem(), fileItem()),
      accepted = true,
      importedCount = 1,
      expectedError = "일부 파일을 삽입하지 못했어요.",
    )
  }

  @Test
  fun mixedFailuresUseAttachmentErrorCopy() = runTest {
    assertCompletionError(
      items = mutableListOf(imageItem(), fileItem()),
      accepted = true,
      importedCount = 0,
      expectedError = "첨부 파일을 삽입하지 못했어요.",
    )
    assertCompletionError(
      items = mutableListOf(imageItem(), fileItem()),
      accepted = true,
      importedCount = 1,
      expectedError = "일부 첨부 파일을 삽입하지 못했어요.",
    )
  }

  @Test
  fun unreadableItemsCountAsGenericAttachmentFailures() = runTest {
    assertCompletionError(
      items = mutableListOf(imageItem()),
      unreadableItemCount = 1,
      accepted = true,
      importedCount = 1,
      expectedError = "일부 첨부 파일을 삽입하지 못했어요.",
    )
  }

  @Test
  fun completeSuccessDoesNotEmitAnAttachmentError() = runTest {
    assertCompletionError(
      items = mutableListOf(imageItem(), fileItem()),
      accepted = true,
      importedCount = 2,
      expectedError = null,
    )
  }

  @Test
  fun staleSessionRejectsAndReleasesCandidates() = runTest {
    var released = false
    val editor = Editor(FakeFfiEditor(), this)
    val session = createTestDocumentEditingSession(editor, this)
    val handler =
      SessionEditorIncomingContentHandler(
        importer = EditorAttachmentImporter { _, _, _, _ -> error("must not import") },
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        isSessionCurrent = { false },
        onAttachmentError = {},
      )

    val handled =
      handler.handleCandidates(
        session,
        IncomingContentCandidates(items = listOf(imageItem { released = true })),
      )

    assertFalse(handled)
    assertTrue(released)
  }

  @Test
  fun staleSessionDoesNotReadClipboard() = runTest {
    var pasteCalls = 0
    val editor = Editor(FakeFfiEditor(), this)
    val session = createTestDocumentEditingSession(editor, this)
    val handler =
      SessionEditorIncomingContentHandler(
        importer = EditorAttachmentImporter { _, _, _, _ -> error("must not import") },
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        isSessionCurrent = { false },
        onAttachmentError = {},
      )
    val clipboard =
      object : Clipboard {
        override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = false

        override suspend fun copy(text: String, mimeType: String): Boolean = false

        override suspend fun copyRichText(html: String, text: String): Boolean = false

        override suspend fun paste(): IncomingContentCandidates? {
          pasteCalls += 1
          return null
        }
      }

    val handled = handler.handleClipboard(session, clipboard, IncomingContentMode.Rich)

    assertFalse(handled)
    assertEquals(0, pasteCalls)
  }

  @Test
  fun sessionReplacementDuringClipboardReadRejectsAndReleasesResult() = runTest {
    var current = true
    var released = false
    val editor = Editor(FakeFfiEditor(), this)
    val session = createTestDocumentEditingSession(editor, this)
    val handler =
      SessionEditorIncomingContentHandler(
        importer = EditorAttachmentImporter { _, _, _, _ -> error("must not import") },
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        isSessionCurrent = { current && it === session },
        onAttachmentError = {},
      )
    val clipboard =
      object : Clipboard {
        override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = false

        override suspend fun copy(text: String, mimeType: String): Boolean = false

        override suspend fun copyRichText(html: String, text: String): Boolean = false

        override suspend fun paste(): IncomingContentCandidates {
          current = false
          return IncomingContentCandidates(items = listOf(imageItem { released = true }))
        }
      }

    val handled = handler.handleClipboard(session, clipboard, IncomingContentMode.Rich)

    assertFalse(handled)
    assertTrue(released)
  }

  private suspend fun TestScope.assertCompletionError(
    items: MutableList<IncomingContentItem>,
    unreadableItemCount: Int = 0,
    accepted: Boolean,
    importedCount: Int,
    expectedError: String?,
  ) {
    val requestedCount = items.size
    val errors = mutableListOf<String>()
    var transferredCount = 0
    lateinit var completeImport: (Int) -> Unit
    val editor = Editor(FakeFfiEditor(), this)
    val session = createTestDocumentEditingSession(editor, this)
    val handler =
      SessionEditorIncomingContentHandler(
        importer =
          EditorAttachmentImporter { importedSession, transferredItems, destination, onCompleted ->
            assertTrue(importedSession === session)
            assertEquals(EditorAttachmentDestination.CurrentSelection, destination)
            transferredCount = transferredItems.size
            completeImport = onCompleted
            transferredItems.forEach { it.file.close() }
            items.clear()
            accepted
          },
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        isSessionCurrent = { it === session },
        onAttachmentError = { error -> errors += error },
      )

    val handled =
      handler.handleCandidates(
        session,
        IncomingContentCandidates(items = items, unreadableItemCount = unreadableItemCount),
      )

    assertEquals(accepted, handled)
    assertEquals(requestedCount, transferredCount)
    assertTrue(errors.isEmpty())

    completeImport(importedCount)

    assertEquals(listOfNotNull(expectedError), errors)
  }

  private fun imageItem(onRelease: () -> Unit = {}): IncomingContentItem =
    IncomingContentItem(
      kind = IncomingContentItem.Kind.Image,
      file =
        PickedFile(
          filename = "image.png",
          mimeType = "image/png",
          size = 1,
          previewModel = Unit,
          imageWidth = 1,
          imageHeight = 1,
          openSource = { Buffer() },
          release = onRelease,
        ),
    )

  private fun fileItem(onRelease: () -> Unit = {}): IncomingContentItem =
    IncomingContentItem(
      kind = IncomingContentItem.Kind.File,
      file =
        PickedFile(
          filename = "document.pdf",
          mimeType = "application/pdf",
          size = 1,
          previewModel = Unit,
          openSource = { Buffer() },
          release = onRelease,
        ),
    )
}
