package co.typie.screen.editor.editor.attachment

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorFileUpload
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.external.EditorImageUpload
import co.typie.editor.ffi.AttachmentPlaceholderKind
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.ImageNodeAttr
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeAttr
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.StateField
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.platform.IncomingContentItem
import co.typie.platform.PickedFile
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import kotlinx.io.Buffer

@OptIn(ExperimentalCoroutinesApi::class)
class DefaultEditorAttachmentImporterTest {
  @Test
  fun cancellationBeforeBackgroundCompletionCleansPendingAndFile() = runTest {
    var released = false
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          listOf(
            EditorEvent.AttachmentPlaceholdersInserted(
              requestId = requestedPlaceholderId(fake),
              nodeIds = listOf(nodeId),
            )
          )
        },
        externalElementsProvider = { listOf(emptyImageElement(nodeId)) },
      )
    val dispatcher = StandardTestDispatcher(testScheduler)
    val editor = Editor(fake, this, dispatcher)
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val backgroundJob = Job()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence = FakePersistence,
        isSessionCurrent = { it === session },
        backgroundScope = CoroutineScope(backgroundJob + dispatcher),
      )

    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = {},
      )

    assertTrue(accepted)
    assertTrue(state.images.uploads.containsKey(nodeId))
    assertFalse(released)

    backgroundJob.cancel()
    advanceUntilIdle()

    assertNull(state.images.uploads[nodeId])
    assertTrue(released)
  }

  @Test
  fun commitsActiveCompositionBeforePlaceholderInsertionThroughImporter() = runTest {
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          if (fake.placeholderRequests().isNotEmpty()) {
            listOf(
              EditorEvent.StateChanged(fields = listOf(StateField.Ime)),
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = listOf(nodeId),
              ),
            )
          } else {
            listOf(EditorEvent.StateChanged(fields = listOf(StateField.Ime)))
          }
        },
        imeProvider = { _, _ ->
          Ime(text = "가", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
        },
        externalElementsProvider = { listOf(emptyImageElement(nodeId)) },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    editor.setImeSessionActive(true)
    editor.refreshImeSnapshot()
    fake.enqueued.clear()
    val session = createTestDocumentEditingSession(editor, this)
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = EditorExternalElementState(),
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence = FakePersistence,
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )

    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem {}),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = {},
      )

    assertTrue(accepted)
    assertEquals(Message.TextInput(listOf(FlatImeOp.CommitAsIs)), fake.enqueued[0])
    val insertion = (fake.enqueued[1] as Message.Insertion).op as InsertionOp.AttachmentPlaceholders
    assertEquals(listOf(AttachmentPlaceholderKind.Image), insertion.kinds)
    advanceUntilIdle()
  }

  @Test
  fun commandStageReturnsAfterPendingInstallationWithoutWaitingForUpload() = runTest {
    var released = false
    val persistenceStarted = CompletableDeferred<Unit>()
    val persistenceGate = CompletableDeferred<Unit>()
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          if (fake.placeholderRequests().isNotEmpty()) {
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = listOf(nodeId),
              )
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = { listOf(emptyImageElement(nodeId)) },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val bringIntoViewRequests = EditorBringIntoViewRequests()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = bringIntoViewRequests,
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset {
              persistenceStarted.complete(Unit)
              persistenceGate.await()
              return FakePersistence.persistImage(file)
            }
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )

    val callbackInvocations = mutableListOf<Int>()
    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertFalse(persistenceStarted.isCompleted)
    assertTrue(accepted)
    assertTrue(callbackInvocations.isEmpty())
    assertTrue(state.images.uploads.containsKey(nodeId))
    assertFalse(released)
    assertEquals(
      EditorBringIntoViewRequests.Request(EditorBringIntoViewTarget.CurrentSelectionHead),
      bringIntoViewRequests.activateForVersion(editor.state.version),
    )

    persistenceStarted.await()
    assertFalse(released)
    persistenceGate.complete(Unit)
    advanceUntilIdle()

    assertEquals(listOf(1), callbackInvocations)
    assertTrue(released)
  }

  @Test
  fun mapsPlaceholderUploadsAndCommitsAssetId() = runTest {
    var released = false
    var tick = 0
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          tick += 1
          if (tick == 1) {
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = "other-request",
                nodeIds = emptyList(),
              ),
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = listOf(nodeId),
              ),
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = { listOf(emptyImageElement(nodeId)) },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence = FakePersistence,
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )

    val callbackInvocations = mutableListOf<Int>()
    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    assertTrue(state.images.uploads.containsKey(nodeId))
    advanceUntilIdle()
    assertEquals(listOf(1), callbackInvocations)
    assertTrue(released)
    assertNull(state.images.uploads[nodeId])
    assertEquals("asset-image", state.images.assets["asset-image"]?.id)
    val committed =
      fake.enqueued
        .filterIsInstance<Message.Node>()
        .map(Message.Node::op)
        .filterIsInstance<NodeOp.SetAttr>()
        .single()
    assertEquals(nodeId, committed.id)
    assertEquals(NodeAttr.Image(ImageNodeAttr.Id("asset-image")), committed.attr)
  }

  @Test
  fun existingImagePlaceholderUsesFirstItemAndInsertsOnlyRemainingPlaceholders() = runTest {
    var firstReleased = false
    var secondReleased = false
    var placeholderRequestHandled = false
    val existingNodeId = "existing-image-node"
    val insertedNodeId = "inserted-image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          if (!placeholderRequestHandled && fake.placeholderRequests().isNotEmpty()) {
            placeholderRequestHandled = true
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = listOf(insertedNodeId),
              )
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = {
          listOf(emptyImageElement(existingNodeId), emptyImageElement(insertedNodeId))
        },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    editor.sync {}
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset =
              EditorImageAsset(
                id = "asset-${file.filename.removeSuffix(".png")}",
                url = "https://example.com/${file.filename}",
                width = 10,
                height = 5,
                ratio = 2.0,
                placeholder = null,
              )
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )
    val callbackInvocations = mutableListOf<Int>()

    val accepted =
      importer.import(
        session = session,
        items =
          listOf(
            imageItem(filename = "first.png") { firstReleased = true },
            imageItem(filename = "second.png") { secondReleased = true },
          ),
        destination =
          EditorAttachmentDestination.ExistingPlaceholder(
            nodeId = existingNodeId,
            expectedKind = IncomingContentItem.Kind.Image,
          ),
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    assertTrue(state.images.uploads.containsKey(existingNodeId))
    assertTrue(state.images.uploads.containsKey(insertedNodeId))
    val placeholderRequests = fake.placeholderRequests()
    assertEquals(1, placeholderRequests.size)
    assertEquals(listOf(AttachmentPlaceholderKind.Image), placeholderRequests.single().kinds)

    advanceUntilIdle()

    assertEquals(listOf(2), callbackInvocations)
    assertTrue(firstReleased)
    assertTrue(secondReleased)
    assertNull(state.images.uploads[existingNodeId])
    assertNull(state.images.uploads[insertedNodeId])
    val commits =
      fake.enqueued
        .filterIsInstance<Message.Node>()
        .map(Message.Node::op)
        .filterIsInstance<NodeOp.SetAttr>()
    assertEquals(
      listOf(
        NodeOp.SetAttr(id = existingNodeId, attr = NodeAttr.Image(ImageNodeAttr.Id("asset-first"))),
        NodeOp.SetAttr(id = insertedNodeId, attr = NodeAttr.Image(ImageNodeAttr.Id("asset-second"))),
      ),
      commits,
    )
  }

  @Test
  fun mixedBatchZipsOrderedKindsAndNodeIds() = runTest {
    val nodeIds = listOf("first-image", "file", "second-image")
    var placeholderRequestHandled = false
    var releasedCount = 0
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          if (!placeholderRequestHandled && fake.placeholderRequests().isNotEmpty()) {
            placeholderRequestHandled = true
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = nodeIds,
              )
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = {
          listOf(
            emptyImageElement(nodeIds[0]),
            emptyFileElement(nodeIds[1]),
            emptyImageElement(nodeIds[2]),
          )
        },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = EditorExternalElementState(),
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset =
              EditorImageAsset(
                id = "asset-${file.filename.removeSuffix(".png")}",
                url = "https://example.com/${file.filename}",
                width = 10,
                height = 5,
                ratio = 2.0,
                placeholder = null,
              )

            override suspend fun persistFile(file: PickedFile): EditorFileAsset =
              EditorFileAsset(
                id = "asset-file",
                name = file.filename,
                url = "https://example.com/file.pdf",
                size = file.size,
              )
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )

    val callbackInvocations = mutableListOf<Int>()
    val accepted =
      importer.import(
        session = session,
        items =
          listOf(
            imageItem(filename = "first.png") { releasedCount += 1 },
            fileItem { releasedCount += 1 },
            imageItem(filename = "second.png") { releasedCount += 1 },
          ),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    assertEquals(
      listOf(
        AttachmentPlaceholderKind.Image,
        AttachmentPlaceholderKind.File,
        AttachmentPlaceholderKind.Image,
      ),
      fake.placeholderRequests().single().kinds,
    )

    advanceUntilIdle()

    assertEquals(listOf(3), callbackInvocations)
    assertEquals(3, releasedCount)
    val committedAssetsByNode =
      fake.enqueued.filterIsInstance<Message.Node>().associate { message ->
        when (val op = message.op) {
          is NodeOp.SetAttr -> op.id to ((op.attr as NodeAttr.Image).attr as ImageNodeAttr.Id).value
          is NodeOp.SetAttrs -> op.id to (op.attrs as PlainNode.File).id
          else -> error("Unexpected node operation: $op")
        }
      }
    assertEquals(
      mapOf(nodeIds[0] to "asset-first", nodeIds[1] to "asset-file", nodeIds[2] to "asset-second"),
      committedAssetsByNode,
    )
  }

  @Test
  fun existingFilePlaceholderPersistsAndCommitsFileAsset() = runTest {
    var released = false
    var persistFileCount = 0
    val nodeId = "file-node"
    val fake = FakeFfiEditor(externalElementsProvider = { listOf(emptyFileElement(nodeId)) })
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    editor.sync {}
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset =
              error("unexpected image persistence")

            override suspend fun persistFile(file: PickedFile): EditorFileAsset {
              persistFileCount += 1
              return EditorFileAsset(
                id = "asset-file",
                name = file.filename,
                url = "https://example.com/file.pdf",
                size = file.size,
              )
            }
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )
    val callbackInvocations = mutableListOf<Int>()

    val accepted =
      importer.import(
        session = session,
        items = listOf(fileItem { released = true }),
        destination =
          EditorAttachmentDestination.ExistingPlaceholder(
            nodeId = nodeId,
            expectedKind = IncomingContentItem.Kind.File,
          ),
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    val pending = state.files.uploads[nodeId]
    assertTrue(pending is EditorFileUpload)
    assertEquals("file.pdf", pending.name)
    assertEquals(42, pending.size)
    assertTrue(fake.placeholderRequests().isEmpty())

    advanceUntilIdle()

    assertEquals(1, persistFileCount)
    assertEquals(listOf(1), callbackInvocations)
    assertTrue(released)
    assertNull(state.files.uploads[nodeId])
    assertEquals("asset-file", state.files.assets["asset-file"]?.id)
    val committed =
      fake.enqueued
        .filterIsInstance<Message.Node>()
        .map(Message.Node::op)
        .filterIsInstance<NodeOp.SetAttrs>()
        .single()
    assertEquals(NodeOp.SetAttrs(id = nodeId, attrs = PlainNode.File(id = "asset-file")), committed)
  }

  @Test
  fun replacingPendingImageDuringPersistenceKeepsReplacementAndSkipsCommit() = runTest {
    var released = false
    val nodeId = "image-node"
    val fake = FakeFfiEditor(externalElementsProvider = { listOf(emptyImageElement(nodeId)) })
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    editor.sync {}
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val replacement =
      EditorImageUpload(previewModel = Unit, name = "replacement.png", width = 20, height = 10)
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset {
              val original = state.images.uploads[nodeId]
              assertTrue(original is EditorImageUpload)
              assertTrue(original !== replacement)
              state.images.uploads[nodeId] = replacement
              return FakePersistence.persistImage(file)
            }
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )
    val callbackInvocations = mutableListOf<Int>()

    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination =
          EditorAttachmentDestination.ExistingPlaceholder(
            nodeId = nodeId,
            expectedKind = IncomingContentItem.Kind.Image,
          ),
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    assertTrue(state.images.uploads[nodeId] !== replacement)

    advanceUntilIdle()

    assertEquals(listOf(0), callbackInvocations)
    assertTrue(released)
    assertTrue(state.images.uploads[nodeId] === replacement)
    assertFalse(fake.enqueued.any { it is Message.Node })
  }

  @Test
  fun persistenceFailureCompletesOnceAndClearsPendingOwnership() = runTest {
    var released = false
    var tick = 0
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          tick += 1
          if (tick == 1) {
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = listOf(nodeId),
              )
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = { listOf(emptyImageElement(nodeId)) },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset =
              error("persistence failed")
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )
    val callbackInvocations = mutableListOf<Int>()

    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    assertTrue(state.images.uploads.containsKey(nodeId))
    advanceUntilIdle()
    assertEquals(listOf(0), callbackInvocations)
    assertTrue(released)
    assertNull(state.images.uploads[nodeId])
    assertFalse(fake.enqueued.any { it is Message.Node })
  }

  @Test
  fun oneOfTwoPersistenceFailuresCompletesOnceWithOneSuccessAndClosesBothFiles() = runTest {
    var successReleased = false
    var failureReleased = false
    var tick = 0
    val nodeIds = listOf("successful-image-node", "failed-image-node")
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          tick += 1
          if (tick == 1) {
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = nodeIds,
              )
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = { nodeIds.map(::emptyImageElement) },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val persistedFilenames = mutableListOf<String>()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset {
              persistedFilenames += file.filename
              if (file.filename == "failed.png") error("persistence failed")
              return FakePersistence.persistImage(file)
            }
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )
    val callbackInvocations = mutableListOf<Int>()

    val accepted =
      importer.import(
        session = session,
        items =
          listOf(
            imageItem(filename = "successful.png") { successReleased = true },
            imageItem(filename = "failed.png") { failureReleased = true },
          ),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    assertTrue(nodeIds.all(state.images.uploads::containsKey))
    advanceUntilIdle()
    assertEquals(listOf(1), callbackInvocations)
    assertEquals(setOf("successful.png", "failed.png"), persistedFilenames.toSet())
    assertEquals(2, persistedFilenames.size)
    assertTrue(successReleased)
    assertTrue(failureReleased)
    assertTrue(nodeIds.none(state.images.uploads::containsKey))
  }

  @Test
  fun sessionBecomingStaleAfterUploadPreventsCommitAndClearsOwnership() = runTest {
    var current = true
    var released = false
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          listOf(
            EditorEvent.AttachmentPlaceholdersInserted(
              requestId = requestedPlaceholderId(fake),
              nodeIds = listOf(nodeId),
            )
          )
        },
        externalElementsProvider = { listOf(emptyImageElement(nodeId)) },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset {
              current = false
              return FakePersistence.persistImage(file)
            }
          },
        isSessionCurrent = { current && it === session },
        backgroundScope = this,
      )

    val callbackInvocations = mutableListOf<Int>()
    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    advanceUntilIdle()
    assertEquals(listOf(0), callbackInvocations)
    assertTrue(released)
    assertNull(state.images.uploads[nodeId])
    assertFalse(fake.enqueued.any { it is Message.Node })
  }

  @Test
  fun placeholderRemovedDuringUploadPreventsCommitAndClosesFile() = runTest {
    var placeholderExists = true
    var placeholderRequestHandled = false
    var released = false
    val nodeId = "image-node"
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          if (!placeholderRequestHandled && fake.placeholderRequests().isNotEmpty()) {
            placeholderRequestHandled = true
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = listOf(nodeId),
              )
            )
          } else {
            emptyList()
          }
        },
        externalElementsProvider = {
          if (placeholderExists) listOf(emptyImageElement(nodeId)) else emptyList()
        },
      )
    lateinit var editor: Editor
    editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val state = EditorExternalElementState()
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = state,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset {
              placeholderExists = false
              editor.sync {}
              return FakePersistence.persistImage(file)
            }
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )

    val callbackInvocations = mutableListOf<Int>()
    val accepted =
      importer.import(
        session = session,
        items = listOf(imageItem { released = true }),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertTrue(accepted)
    advanceUntilIdle()

    assertEquals(listOf(0), callbackInvocations)
    assertTrue(released)
    assertNull(state.images.uploads[nodeId])
    assertFalse(fake.enqueued.any { it is Message.Node })
  }

  @Test
  fun invalidImageIsRejectedBeforeAPlaceholderIsInserted() = runTest {
    var released = false
    val fake = FakeFfiEditor()
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    val session = createTestDocumentEditingSession(editor, this)
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = EditorExternalElementState(),
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence = FakePersistence,
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )
    val invalidImage =
      IncomingContentItem(
        kind = IncomingContentItem.Kind.Image,
        file =
          PickedFile(
            filename = "broken.png",
            mimeType = "image/png",
            size = 1,
            previewModel = Unit,
            openSource = { Buffer() },
            release = { released = true },
          ),
      )

    val callbackInvocations = mutableListOf<Int>()
    val accepted =
      importer.import(
        session = session,
        items = listOf(invalidImage),
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { importedCount -> callbackInvocations += importedCount },
      )

    assertFalse(accepted)
    advanceUntilIdle()
    assertEquals(listOf(0), callbackInvocations)
    assertTrue(released)
    assertTrue(fake.placeholderRequests().isEmpty())
  }

  @Test
  fun missingPlaceholderResultDoesNotStartUploadsAndClosesFiles() = runTest {
    assertInvalidPlaceholderResultDoesNotStartUploads(nodeIds = null)
  }

  @Test
  fun placeholderCountMismatchDoesNotStartUploadsAndClosesFiles() = runTest {
    assertInvalidPlaceholderResultDoesNotStartUploads(nodeIds = listOf("only-one-node"))
  }

  private suspend fun TestScope.assertInvalidPlaceholderResultDoesNotStartUploads(
    nodeIds: List<String>?
  ) {
    var releasedCount = 0
    var persistenceCount = 0
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        onTick = {
          if (fake.placeholderRequests().isEmpty() || nodeIds == null) {
            emptyList()
          } else {
            listOf(
              EditorEvent.AttachmentPlaceholdersInserted(
                requestId = requestedPlaceholderId(fake),
                nodeIds = nodeIds,
              )
            )
          }
        }
      )
    val dispatcher = StandardTestDispatcher(testScheduler)
    val editorScope = CoroutineScope(SupervisorJob() + dispatcher)
    val editor = Editor(fake, editorScope, dispatcher)
    val session = createTestDocumentEditingSession(editor, this)
    val importer =
      DefaultEditorAttachmentImporter(
        externalElementState = EditorExternalElementState(),
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        persistence =
          object : EditorAttachmentPersistence by FakePersistence {
            override suspend fun persistImage(file: PickedFile): EditorImageAsset {
              persistenceCount += 1
              return FakePersistence.persistImage(file)
            }
          },
        isSessionCurrent = { it === session },
        backgroundScope = this,
      )

    try {
      assertFailsWith<IllegalStateException> {
        importer.import(
          session = session,
          items =
            listOf(
              imageItem(filename = "first.png") { releasedCount += 1 },
              imageItem(filename = "second.png") { releasedCount += 1 },
            ),
          destination = EditorAttachmentDestination.CurrentSelection,
          onCompleted = {},
        )
      }
    } finally {
      editorScope.cancel()
    }

    assertEquals(0, persistenceCount)
    assertEquals(2, releasedCount)
  }

  private fun requestedPlaceholderId(fake: FakeFfiEditor?): String {
    return fake?.placeholderRequests()?.lastOrNull()?.requestId
      ?: error("Placeholder request was not enqueued")
  }

  private fun FakeFfiEditor.placeholderRequests(): List<InsertionOp.AttachmentPlaceholders> =
    enqueued
      .filterIsInstance<Message.Insertion>()
      .map(Message.Insertion::op)
      .filterIsInstance<InsertionOp.AttachmentPlaceholders>()

  private fun emptyImageElement(nodeId: String): ExternalElement =
    ExternalElement(
      pageIdx = 0,
      node = nodeId,
      bounds = Rect(0f, 0f, 1f, 1f),
      isSelected = false,
      data = ExternalElementData.Image(id = null, proportion = 100),
    )

  private fun emptyFileElement(nodeId: String): ExternalElement =
    ExternalElement(
      pageIdx = 0,
      node = nodeId,
      bounds = Rect(0f, 0f, 1f, 1f),
      isSelected = false,
      data = ExternalElementData.File(id = null),
    )

  private fun imageItem(
    filename: String = "image.png",
    onRelease: () -> Unit,
  ): IncomingContentItem =
    IncomingContentItem(
      kind = IncomingContentItem.Kind.Image,
      file =
        PickedFile(
          filename = filename,
          mimeType = "image/png",
          size = 1,
          previewModel = Unit,
          imageWidth = 10,
          imageHeight = 5,
          openSource = { Buffer() },
          release = onRelease,
        ),
    )

  private fun fileItem(onRelease: () -> Unit): IncomingContentItem =
    IncomingContentItem(
      kind = IncomingContentItem.Kind.File,
      file =
        PickedFile(
          filename = "file.pdf",
          mimeType = "application/pdf",
          size = 42,
          previewModel = Unit,
          openSource = { Buffer() },
          release = onRelease,
        ),
    )

  private object FakePersistence : EditorAttachmentPersistence {
    override suspend fun persistImage(file: PickedFile): EditorImageAsset =
      EditorImageAsset(
        id = "asset-image",
        url = "https://example.com/image.png",
        width = 10,
        height = 5,
        ratio = 2.0,
        placeholder = null,
      )

    override suspend fun persistFile(file: PickedFile): EditorFileAsset = error("unexpected file")
  }
}
