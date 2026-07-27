package co.typie.screen.editor.editor.attachment

import co.typie.domain.blob.blobUploadForm
import co.typie.domain.blob.normalizeBlobContentType
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.external.EditorAssetPendingMeta
import co.typie.editor.external.EditorAssetResolution
import co.typie.editor.external.EditorAttachmentFailureStage
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
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
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.editor.sync.ws.WsAssetItem
import co.typie.graphql.TypieError
import co.typie.platform.IncomingContentItem
import co.typie.platform.PickedFile
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.io.Buffer
import kotlinx.io.RawSource
import kotlinx.io.Source
import kotlinx.io.buffered

@OptIn(ExperimentalCoroutinesApi::class)
class DefaultEditorAttachmentImporterTest {
  @Test
  fun fillsAnExistingPlaceholderWithASetAttrFlushOnlyAndEnqueuesNothingOnCompletion() = runTest {
    val fixture = Fixture(this, initialElements = listOf(imageElement("existing")))
    var messagesAtReserve = -1
    fixture.persistence.onReserve = { _, items ->
      messagesAtReserve = fixture.fake.enqueued.size
      items.map { fixture.persistence.reservationFor(it) }
    }

    val item = fixture.imageItem("cover.png", mimeType = "image/png")
    val imported = fixture.importInto("existing", IncomingContentItem.Kind.Image, listOf(item))
    runCurrent()

    assertEquals(1, imported)
    assertEquals(0, messagesAtReserve)
    assertEquals(
      listOf(
        EditorAttachmentReserveItem(
          kind = IncomingContentItem.Kind.Image,
          name = "cover.png",
          format = "image/png",
          size = 1,
          assetId = null,
        )
      ),
      fixture.persistence.reserveCalls.single().second,
    )
    assertEquals(listOf("node"), fixture.document.flushLog)
    assertEquals(
      listOf(NodeOp.SetAttr(id = "existing", attr = NodeAttr.Image(ImageNodeAttr.Id("IMG1")))),
      fixture.nodeOps(),
    )
    assertEquals(listOf("IMG1" to "n1"), fixture.persistence.finalizeImageCalls)
    assertEquals("IMG1", fixture.state.images.assets["IMG1"]?.id)
    assertTrue(fixture.state.images.uploads.isEmpty())
    assertTrue(fixture.released.contains("cover.png"))
  }

  @Test
  fun writesAFileIdThroughSetAttrsAndCachesTheAsset() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))

    val imported =
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(fixture.fileItem("a.pdf")),
      )
    runCurrent()

    assertEquals(1, imported)
    assertEquals(
      listOf(NodeOp.SetAttrs(id = "existing", attrs = PlainNode.File(id = "FILE1"))),
      fixture.nodeOps(),
    )
    assertEquals("FILE1", fixture.state.files.assets["FILE1"]?.id)
    assertEquals(listOf("FILE1" to "n1"), fixture.persistence.finalizeFileCalls)
  }

  @Test
  fun insertsPlaceholdersInOneFlushAndRecordsEachReservedIdInALaterFlush() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a", "node-b")))
    var messagesAtReserve = -1
    fixture.persistence.onReserve = { _, items ->
      messagesAtReserve = fixture.fake.enqueued.size
      items.map { fixture.persistence.reservationFor(it) }
    }

    val imported =
      fixture.importAtSelection(listOf(fixture.imageItem("a.png"), fixture.imageItem("b.png")))
    runCurrent()

    assertEquals(2, imported)
    assertEquals(0, messagesAtReserve)
    assertEquals(listOf("insertion", "node", "node"), fixture.document.flushLog)
    assertEquals(
      listOf(
        NodeOp.SetAttr(id = "node-a", attr = NodeAttr.Image(ImageNodeAttr.Id("IMG1"))),
        NodeOp.SetAttr(id = "node-b", attr = NodeAttr.Image(ImageNodeAttr.Id("IMG2"))),
      ),
      fixture.nodeOps(),
    )
  }

  @Test
  fun mixedBatchZipsOrderedKindsAndNodeIds() = runTest {
    val nodeIds = listOf("first-image", "file-node", "second-image")
    val fixture = Fixture(this, receipts = listOf(nodeIds))

    val imported =
      fixture.importAtSelection(
        listOf(
          fixture.imageItem("first.png"),
          fixture.fileItem("doc.pdf"),
          fixture.imageItem("second.png"),
        )
      )
    runCurrent()

    assertEquals(3, imported)
    assertEquals(
      listOf(
        AttachmentPlaceholderKind.Image,
        AttachmentPlaceholderKind.File,
        AttachmentPlaceholderKind.Image,
      ),
      fixture.placeholderRequests().single().kinds,
    )
    assertEquals(
      mapOf("first-image" to "IMG1", "file-node" to "FILE2", "second-image" to "IMG3"),
      fixture.committedAssetIdsByNode(),
    )
  }

  @Test
  fun keepsTheDocumentUntouchedWhenTheReservationFailsAndReleasesEverySource() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a", "node-b")))
    fixture.persistence.onReserve = { _, _ -> throw TypieError("asset_upload_in_progress", null) }

    val imported =
      fixture.importAtSelection(listOf(fixture.imageItem("a.png"), fixture.fileItem("b.pdf")))
    runCurrent()

    assertEquals(0, imported)
    assertTrue(fixture.fake.enqueued.isEmpty())
    assertTrue(fixture.document.flushLog.isEmpty())
    assertTrue(fixture.persistence.uploadCalls.isEmpty())
    assertTrue(fixture.state.images.uploads.isEmpty())
    assertEquals(setOf("a.png", "b.pdf"), fixture.released.toSet())
  }

  @Test
  fun aCancelledPickerReservesNothingAndReportsZero() = runTest {
    val fixture = Fixture(this)

    val imported = fixture.importAtSelection(emptyList())
    runCurrent()

    assertEquals(0, imported)
    assertTrue(fixture.persistence.reserveCalls.isEmpty())
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  @Test
  fun anInvalidImageIsRejectedBeforeAnythingIsReserved() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a")))

    val imported =
      fixture.importAtSelection(
        listOf(fixture.imageItem("broken.png", width = null), fixture.imageItem("good.png"))
      )
    runCurrent()

    assertEquals(1, imported)
    assertEquals(
      listOf("good.png"),
      fixture.persistence.reserveCalls.single().second.map { it.name },
    )
    assertTrue(fixture.released.contains("broken.png"))
  }

  @Test
  fun refusesASecondImportForTheSamePlaceholderWhileItsReservationIsInFlight() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onReserve = { _, items ->
      gate.await()
      items.map { fixture.persistence.reservationFor(it) }
    }

    val first = backgroundScope.async {
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(fixture.fileItem("a.pdf")),
      )
    }
    runCurrent()

    val second =
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(fixture.fileItem("b.pdf")),
      )
    assertEquals(0, second)

    gate.complete(Unit)
    runCurrent()

    assertEquals(1, first.await())
    assertEquals(1, fixture.persistence.reserveCalls.size)
    assertEquals(1, fixture.nodeOps().size)
  }

  @Test
  fun chunksAReservationLargerThanTheServerItemCap() = runTest {
    val nodeIds = List(5) { "n$it" }
    val fixture = Fixture(this, receipts = listOf(nodeIds), maxReserveItems = 2)

    val imported = fixture.importAtSelection(List(5) { fixture.fileItem("$it.pdf") })
    runCurrent()

    assertEquals(5, imported)
    assertEquals(listOf(2, 2, 1), fixture.persistence.reserveCalls.map { it.second.size })
    assertEquals(5, fixture.nodeOps().size)
  }

  @Test
  fun sendsAssetFailedForATransferFailureAndMarksTheLocalAttemptAsATransferFailure() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    assertEquals(listOf("doc" to listOf(WsAssetItem("FILE1", "n1"))), fixture.assetFailed)
    assertEquals(
      EditorAttachmentFailureStage.Transfer,
      fixture.state.files.uploads["existing"]?.failedStage,
    )
    assertTrue(fixture.persistence.finalizeFileCalls.isEmpty())
    assertTrue(fixture.nodeOps().none { it is NodeOp.Delete })
  }

  @Test
  fun neverSendsAssetFailedForAnUncertainFinalizeFailureAndRetriesTheSameNonce() = runTest {
    val fixture =
      Fixture(
        this,
        initialElements = listOf(fileElement("existing")),
        finalizeRetryDelaysMs = listOf(1L, 1L),
      )
    fixture.persistence.onFinalizeFile = { _, _ ->
      throw TypieError("asset_finalize_retryable", null)
    }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(3, fixture.persistence.finalizeFileCalls.size)
    assertTrue(fixture.persistence.finalizeFileCalls.all { it.second == "n1" })
    assertTrue(fixture.assetFailed.isEmpty())
    assertEquals(
      EditorAttachmentFailureStage.FinalizeRetryable,
      fixture.state.files.uploads["existing"]?.failedStage,
    )
  }

  @Test
  fun stopsImmediatelyOnADeterministicFinalizeRejectionWithoutSendingAssetFailed() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onFinalizeFile = { _, _ ->
      throw TypieError("asset_finalize_rejected", null)
    }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    assertEquals(1, fixture.persistence.finalizeFileCalls.size)
    assertTrue(fixture.assetFailed.isEmpty())
    assertEquals(
      EditorAttachmentFailureStage.FinalizeRejected,
      fixture.state.files.uploads["existing"]?.failedStage,
    )
  }

  @Test
  fun fallsBackToAFreshReservationWhenFinalizeReportsAnExpiredLease() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    var attempts = 0
    fixture.persistence.onFinalizeFile = { assetId, _ ->
      attempts += 1
      if (attempts == 1) throw TypieError("asset_upload_expired", null)
      fixture.persistence.fileAsset(assetId)
    }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(listOf("FILE1" to "n1"), fixture.persistence.abandonCalls)
    assertEquals(2, fixture.persistence.reserveCalls.size)
    assertEquals(listOf("FILE1"), fixture.persistence.reserveCalls[1].second.map { it.assetId })
    assertEquals(2, fixture.persistence.uploadCalls.size)
    assertEquals(listOf("n1", "n2"), fixture.persistence.finalizeFileCalls.map { it.second })
    assertEquals(1, fixture.nodeOps().size)
  }

  @Test
  fun oneFailingChildDoesNotCancelItsSiblings() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a", "node-b")))
    fixture.persistence.onUpload = { _, source ->
      if (source.filename == "bad.pdf") throw IllegalStateException("transfer failed")
    }

    fixture.importAtSelection(listOf(fixture.fileItem("bad.pdf"), fixture.fileItem("good.pdf")))
    runCurrent()

    assertEquals(
      EditorAttachmentFailureStage.Transfer,
      fixture.state.files.uploads["node-a"]?.failedStage,
    )
    assertEquals("FILE2", fixture.state.files.assets["FILE2"]?.id)
    assertNull(fixture.state.files.uploads["node-b"])
    assertEquals(listOf("doc" to listOf(WsAssetItem("FILE1", "n1"))), fixture.assetFailed)
  }

  @Test
  fun abandonsTheOldLeaseBeforeReReservingAfterATransferFailure() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ ->
      fixture.persistence.onUpload = { _, _ -> }
      throw IllegalStateException("network down")
    }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    assertTrue(fixture.importer.retry("FILE1"))
    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(
      listOf("reserve", "abandon", "reserve"),
      fixture.persistence.order.filter { it == "reserve" || it == "abandon" },
    )
    assertEquals(listOf("FILE1" to "n1"), fixture.persistence.abandonCalls)
    assertEquals(listOf("FILE1" to "n2"), fixture.persistence.finalizeFileCalls)
    assertEquals(1, fixture.nodeOps().size)
  }

  @Test
  fun reFinalizesTheSameNonceWithoutAbandoningAfterAFinalizeFailure() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onFinalizeFile = { _, _ ->
      fixture.persistence.onFinalizeFile = null
      throw TypieError("asset_finalize_rejected", null)
    }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    assertTrue(fixture.importer.retry("FILE1"))
    runCurrent()

    assertTrue(fixture.persistence.abandonCalls.isEmpty())
    assertEquals(1, fixture.persistence.reserveCalls.size)
    assertEquals(1, fixture.persistence.uploadCalls.size)
    assertEquals(listOf("n1", "n1"), fixture.persistence.finalizeFileCalls.map { it.second })
  }

  @Test
  fun refusesToRetryAnAttemptThatIsNotInAFailedState() = runTest {
    val fixture = Fixture(this)

    assertFalse(fixture.importer.retry("IMG-unknown"))
    assertFalse(fixture.importer.canRetry("IMG-unknown"))
  }

  @Test
  fun retryReusesTheRetainedSourceAndIsRefusedOnceThatSourceIsGone() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    assertTrue(fixture.importer.canRetry("FILE1"))
    assertFalse(fixture.released.contains("a.pdf"))
    assertTrue(fixture.importer.retry("FILE1"))
    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(
      listOf("a.pdf", "a.pdf"),
      fixture.persistence.uploadCalls.map { it.second.filename },
    )

    fixture.importer.cancelNode("existing")
    assertTrue(fixture.released.contains("a.pdf"))
    assertFalse(fixture.importer.canRetry("FILE1"))
    assertFalse(fixture.importer.retry("FILE1"))
  }

  @Test
  fun reReservesAsSoonAsThePullReportsTheIdAsMissing() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.persistence.onAbandon = { _, _ -> false }
    fixture.onRefresh = { ids ->
      ids.forEach { fixture.state.resolutions[it] = EditorAssetResolution.Missing }
    }
    fixture.persistence.onUpload = null

    assertTrue(fixture.importer.retry("FILE1"))
    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(listOf(listOf("FILE1")), fixture.refreshed)
    assertEquals(1, fixture.persistence.abandonCalls.size)
    assertEquals(2, fixture.persistence.reserveCalls.size)
  }

  @Test
  fun waitsAndRetriesAbandonWhileThePullStillReportsTheIdAsPending() = runTest {
    val fixture =
      Fixture(this, initialElements = listOf(fileElement("existing")), settleAttempts = 3)
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    var abandonAttempts = 0
    fixture.persistence.onAbandon = { _, _ ->
      abandonAttempts += 1
      abandonAttempts >= 3
    }
    fixture.onRefresh = { ids ->
      ids.forEach {
        fixture.state.resolutions[it] =
          EditorAssetResolution.Pending(EditorAssetPendingMeta("file", "a.pdf", 1L))
      }
    }
    fixture.persistence.onUpload = null

    assertTrue(fixture.importer.retry("FILE1"))
    advanceTimeBy(5_000)
    runCurrent()

    assertEquals(3, fixture.persistence.abandonCalls.size)
    assertEquals(2, fixture.persistence.reserveCalls.size)
  }

  @Test
  fun retiresTheLocalAttemptWhenThePullReportsTheIdAsAlreadyReady() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.persistence.onAbandon = { _, _ -> false }
    fixture.onRefresh = { ids ->
      ids.forEach { fixture.state.files.assets[it] = fixture.persistence.fileAsset(it) }
    }

    assertTrue(fixture.importer.retry("FILE1"))
    advanceTimeBy(5_000)
    runCurrent()

    assertEquals(1, fixture.persistence.reserveCalls.size)
    assertNull(fixture.state.files.uploads["existing"])
    assertTrue(fixture.released.contains("a.pdf"))
  }

  @Test
  fun givesUpWaitingAfterABoundedNumberOfAttemptsInsteadOfBlockingForever() = runTest {
    val fixture =
      Fixture(this, initialElements = listOf(fileElement("existing")), settleAttempts = 2)
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.persistence.onAbandon = { _, _ -> false }
    fixture.persistence.onUpload = null

    assertTrue(fixture.importer.retry("FILE1"))
    advanceTimeBy(5_000)
    runCurrent()

    assertEquals(2, fixture.persistence.abandonCalls.size)
    assertEquals(2, fixture.persistence.reserveCalls.size)
  }

  @Test
  fun reselectAbandonsTheLiveLeaseBeforeReservingTheSameIdAndTouchesNoDocument() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()
    val messagesBefore = fixture.fake.enqueued.size
    fixture.persistence.onUpload = null

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "FILE1",
        nodeId = "existing",
        kind = IncomingContentItem.Kind.File,
        file = pickedFile("b.pdf", fixture.released),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(
      listOf("reserve", "abandon", "reserve"),
      fixture.persistence.order.filter { it == "reserve" || it == "abandon" },
    )
    assertEquals(
      listOf("FILE1" to "b.pdf"),
      fixture.persistence.reserveCalls.last().second.map { it.assetId to it.name },
    )
    assertEquals(messagesBefore, fixture.fake.enqueued.size)
    assertTrue(fixture.released.contains("a.pdf"))
  }

  @Test
  fun reselectDiscardsTheNewlyPickedFileWhenAbandonRevealsACompletedAsset() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.persistence.onAbandon = { _, _ -> false }
    fixture.onRefresh = { ids ->
      ids.forEach { fixture.state.files.assets[it] = fixture.persistence.fileAsset(it) }
    }
    val reserveCallsBefore = fixture.persistence.reserveCalls.size

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "FILE1",
        nodeId = "existing",
        kind = IncomingContentItem.Kind.File,
        file = pickedFile("b.pdf", fixture.released),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    advanceTimeBy(5_000)
    runCurrent()

    assertEquals(reserveCallsBefore, fixture.persistence.reserveCalls.size)
    assertNull(fixture.state.files.uploads["existing"])
    assertTrue(fixture.released.contains("b.pdf"))
  }

  @Test
  fun reselectReclaimsAnUnresolvedNodeThatHasNoLocalAttemptWithoutAbandoningAnything() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("node-a", id = "FILE7")))
    fixture.state.resolutions["FILE7"] = EditorAssetResolution.Missing

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "FILE7",
        nodeId = "node-a",
        kind = IncomingContentItem.Kind.File,
        file = pickedFile("b.pdf", fixture.released),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    runCurrent()

    assertTrue(fixture.persistence.abandonCalls.isEmpty())
    assertEquals(
      listOf("FILE7"),
      fixture.persistence.reserveCalls.single().second.map { it.assetId },
    )
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  @Test
  fun reselectPullsInsteadOfFailingWhenTheReclaimedIdWasCompletedMeanwhile() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("node-a", id = "FILE7")))
    fixture.state.resolutions["FILE7"] = EditorAssetResolution.Missing
    fixture.persistence.onReserve = { _, _ -> throw TypieError("asset_already_exists", null) }

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "FILE7",
        nodeId = "node-a",
        kind = IncomingContentItem.Kind.File,
        file = pickedFile("b.pdf", fixture.released),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    runCurrent()

    // 실패가 아니라 "그 사이에 해석됐다" — 실패 카드 대신 pull로 완성 자산을 받는다.
    assertEquals(0, fixture.reselectFailures)
    assertEquals(listOf(listOf("FILE7")), fixture.refreshed)
    assertTrue(fixture.state.files.uploads.isEmpty())
    assertTrue(fixture.released.contains("b.pdf"))
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  @Test
  fun retiresTheRunWhenTheExpiredLeaseTurnsOutToBeACompletedAsset() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onFinalizeFile = { _, _ -> throw TypieError("asset_upload_expired", null) }
    fixture.persistence.onReserve = { _, items ->
      fixture.persistence.onReserve = { _, _ -> throw TypieError("asset_already_exists", null) }
      items.map { fixture.persistence.reservationFor(it) }
    }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(listOf(listOf("FILE1")), fixture.refreshed)
    assertNull(fixture.state.files.uploads["existing"])
  }

  @Test
  fun commitsALiveCompositionBeforeRecordingTheReservedId() = runTest {
    val fixture = Fixture(this, initialElements = listOf(imageElement("existing")))
    fixture.fake.imeProvider = { _, _ ->
      Ime(text = "ㄱ", windowStart = 0, selection = ImeRange(1, 1), composing = ImeRange(0, 1))
    }
    fixture.editor.setImeSessionActive(true)
    fixture.editor.refreshImeSnapshot()
    runCurrent()

    fixture.importInto(
      "existing",
      IncomingContentItem.Kind.Image,
      listOf(fixture.imageItem("cover.png")),
    )
    runCurrent()

    // 조합 중인 IME를 커밋하지 않고 SetAttr을 밀면 조합 문자가 노드 경계에서 유실된다.
    assertEquals(
      listOf(
        Message.TextInput(listOf(FlatImeOp.CommitAsIs)),
        Message.Node(
          NodeOp.SetAttr(id = "existing", attr = NodeAttr.Image(ImageNodeAttr.Id("IMG1")))
        ),
      ),
      fixture.fake.enqueued.toList(),
    )
  }

  @Test
  fun beatsEveryTwentySecondsForActiveTransfersAndStopsOnceNoneRemain() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    assertEquals(20_000L, AssetHeartbeatIntervalMs)
    advanceTimeBy(AssetHeartbeatIntervalMs + 1)
    runCurrent()
    assertEquals(listOf("doc" to listOf(WsAssetItem("FILE1", "n1"))), fixture.heartbeats)

    gate.complete(Unit)
    runCurrent()
    advanceTimeBy(AssetHeartbeatIntervalMs * 3)
    runCurrent()

    assertEquals(1, fixture.heartbeats.size)
  }

  @Test
  fun doesNotBeatForAnAttemptThatHasAlreadyFailed() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()
    advanceTimeBy(AssetHeartbeatIntervalMs * 3)
    runCurrent()

    assertTrue(fixture.heartbeats.isEmpty())
  }

  @Test
  fun beatsOnlyWhileTheScreenIsInTheForeground() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.importer.setForeground(false)
    advanceTimeBy(AssetHeartbeatIntervalMs * 3)
    runCurrent()
    assertTrue(fixture.heartbeats.isEmpty())

    fixture.importer.setForeground(true)
    advanceTimeBy(AssetHeartbeatIntervalMs + 1)
    runCurrent()
    assertEquals(1, fixture.heartbeats.size)

    gate.complete(Unit)
    runCurrent()
  }

  @Test
  fun returningToTheForegroundMarksATransferThatDiedInTheBackgroundAsFailed() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()
    assertNull(fixture.state.files.uploads["existing"]?.failedStage)

    fixture.importer.setForeground(false)
    gate.completeExceptionally(CancellationException("app suspended"))
    runCurrent()
    assertNull(fixture.state.files.uploads["existing"]?.failedStage)

    fixture.importer.setForeground(true)

    assertEquals(
      EditorAttachmentFailureStage.Transfer,
      fixture.state.files.uploads["existing"]?.failedStage,
    )
    assertTrue(fixture.assetFailed.isEmpty())
  }

  @Test
  fun disposeCancelsTransfersReleasesSourcesAndSendsNoAssetFailed() = runTest {
    val fixture = Fixture(this, initialElements = listOf(imageElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto(
      "existing",
      IncomingContentItem.Kind.Image,
      listOf(fixture.imageItem("picture.png")),
    )
    runCurrent()
    assertNotNull(fixture.state.images.uploads["existing"])

    fixture.importer.dispose()
    runCurrent()
    advanceTimeBy(AssetHeartbeatIntervalMs * 3)
    runCurrent()

    assertTrue(fixture.state.images.uploads.isEmpty())
    assertTrue(fixture.released.contains("picture.png"))
    assertTrue(fixture.heartbeats.isEmpty())
    assertTrue(fixture.assetFailed.isEmpty())
  }

  @Test
  fun retiresAFailedCardAndItsRetainedSourceWhenTheServerReportsTheAssetAsReady() = runTest {
    val fixture = Fixture(this, initialElements = listOf(imageElement("existing")))
    fixture.persistence.onFinalizeImage = { _, _ ->
      throw TypieError("asset_finalize_retryable", null)
    }

    fixture.importInto(
      "existing",
      IncomingContentItem.Kind.Image,
      listOf(fixture.imageItem("picture.png")),
    )
    runCurrent()
    assertEquals(
      EditorAttachmentFailureStage.FinalizeRetryable,
      fixture.state.images.uploads["existing"]?.failedStage,
    )

    fixture.importer.retireCompleted(listOf("IMG1"))

    assertTrue(fixture.state.images.uploads.isEmpty())
    assertTrue(fixture.released.contains("picture.png"))
    assertFalse(fixture.importer.canRetry("IMG1"))
  }

  @Test
  fun dropsTheLocalAttemptForANodeTheUserDeleted() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.importer.cancelNode("existing")

    assertTrue(fixture.state.files.uploads.isEmpty())
    gate.complete(Unit)
    runCurrent()
    assertTrue(fixture.persistence.finalizeFileCalls.isEmpty())
  }

  @Test
  fun startsAtMostFiveTransfersAtATime() = runTest {
    val fixture = Fixture(this, receipts = listOf(List(7) { "node-$it" }))
    val gate = CompletableDeferred<Unit>()
    var active = 0
    var maxActive = 0
    fixture.persistence.onUpload = { _, _ ->
      active += 1
      maxActive = maxOf(maxActive, active)
      gate.await()
      active -= 1
    }

    fixture.importAtSelection(List(7) { fixture.fileItem("$it.pdf") })
    runCurrent()

    assertEquals(5, maxActive)
    gate.complete(Unit)
    runCurrent()
    assertEquals(5, maxActive)
    assertEquals(7, fixture.nodeOps().size)
  }

  @Test
  fun finishesTheUploadEvenWhenTheNodeIsRemovedAndEnqueuesNothingMore() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.document.elements.clear()
    gate.complete(Unit)
    runCurrent()

    assertEquals("FILE1", fixture.state.files.assets["FILE1"]?.id)
    assertEquals(1, fixture.nodeOps().size)
    assertTrue(fixture.nodeOps().single() is NodeOp.SetAttrs)
  }

  @Test
  fun aStaleSessionAfterTheReservationLeavesTheDocumentUntouched() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onReserve = { _, items ->
      fixture.sessionCurrent = false
      items.map { fixture.persistence.reservationFor(it) }
    }

    val imported =
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(fixture.fileItem("a.pdf")),
      )
    runCurrent()

    assertEquals(0, imported)
    assertTrue(fixture.fake.enqueued.isEmpty())
    assertTrue(fixture.persistence.uploadCalls.isEmpty())
    assertTrue(fixture.released.contains("a.pdf"))
  }

  @Test
  fun anUnusableReceiptReservesButNeitherRecordsAnIdNorStartsATransfer() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("only-one")))

    val imported =
      fixture.importAtSelection(listOf(fixture.fileItem("a.pdf"), fixture.fileItem("b.pdf")))
    runCurrent()

    assertEquals(0, imported)
    assertEquals(1, fixture.persistence.reserveCalls.size)
    assertTrue(fixture.nodeOps().isEmpty())
    assertTrue(fixture.persistence.uploadCalls.isEmpty())
    assertEquals(setOf("a.pdf", "b.pdf"), fixture.released.toSet())
  }

  @Test
  fun normalizesABlankContentTypeToTheSameValueInReserveTheFormAndTheFilePart() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val source = pickedFile("a.bin", fixture.released, mimeType = "   ")

    fixture.importInto(
      "existing",
      IncomingContentItem.Kind.File,
      listOf(IncomingContentItem(kind = IncomingContentItem.Kind.File, file = source)),
    )
    runCurrent()

    val reservedFormat = fixture.persistence.reserveCalls.single().second.single().format
    val form = blobUploadForm(emptyMap(), source)

    assertEquals("application/octet-stream", reservedFormat)
    assertEquals(reservedFormat, form.fields["Content-Type"])
    assertEquals(reservedFormat, form.filePartContentType)
    assertEquals(reservedFormat, normalizeBlobContentType(source.mimeType))
  }

  @Test
  fun measuresTheSourceWhenThePickerDoesNotReportASizeInsteadOfReservingZero() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }
    val source = pickedFile("a.bin", fixture.released, size = null, sourceBytes = 1234)

    val imported =
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(IncomingContentItem(kind = IncomingContentItem.Kind.File, file = source)),
      )
    runCurrent()

    assertEquals(1, imported)
    assertEquals(1234L, fixture.persistence.reserveCalls.single().second.single().size)
    assertEquals(1234L, fixture.state.files.uploads.getValue("existing").size)

    gate.complete(Unit)
    runCurrent()
  }

  @Test
  fun measuresASourceThatSpansManyReadsInsteadOfOnlyItsFirstChunk() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val source = pickedFile("big.bin", fixture.released, size = null, sourceBytes = 300_000)

    val imported =
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(IncomingContentItem(kind = IncomingContentItem.Kind.File, file = source)),
      )
    runCurrent()

    assertEquals(1, imported)
    assertEquals(300_000L, fixture.persistence.reserveCalls.single().second.single().size)
  }

  @Test
  fun cancellingAnImportInFlightReleasesEverySourceItStillOwns() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a", "node-b")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onReserve = { _, items ->
      gate.await()
      items.map { fixture.persistence.reservationFor(it) }
    }

    val importJob = backgroundScope.launch {
      fixture.importAtSelection(listOf(fixture.fileItem("a.pdf"), fixture.fileItem("b.pdf")))
    }
    runCurrent()
    assertTrue(fixture.released.isEmpty())

    importJob.cancel()
    runCurrent()

    assertEquals(setOf("a.pdf", "b.pdf"), fixture.released.toSet())
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  @Test
  fun aReselectWhoseScopeDiesBeforeItRunsStillReleasesThePickedFile() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("node-a", id = "FILE7")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onReserve = { _, items ->
      gate.await()
      items.map { fixture.persistence.reservationFor(it) }
    }

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "FILE7",
        nodeId = "node-a",
        kind = IncomingContentItem.Kind.File,
        file = pickedFile("b.pdf", fixture.released),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    runCurrent()
    assertTrue(fixture.released.isEmpty())

    fixture.backgroundJob.cancel()
    runCurrent()

    assertTrue(fixture.released.contains("b.pdf"))
  }

  @Test
  fun rejectsAnItemWhoseSizeCannotBeResolvedBeforeReservingAnything() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val source = pickedFile("a.bin", fixture.released, size = null, sourceBytes = null)

    val imported =
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(IncomingContentItem(kind = IncomingContentItem.Kind.File, file = source)),
      )
    runCurrent()

    assertEquals(0, imported)
    assertTrue(fixture.persistence.reserveCalls.isEmpty())
    assertTrue(fixture.fake.enqueued.isEmpty())
    assertTrue(fixture.released.contains("a.bin"))
  }

  @Test
  fun reselectReportsAFailureWhenTheNewlyPickedItemCannotBePrepared() = runTest {
    val fixture = Fixture(this, initialElements = listOf(imageElement("node-a", id = "IMG7")))

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "IMG7",
        nodeId = "node-a",
        kind = IncomingContentItem.Kind.Image,
        file = pickedFile("broken.png", fixture.released, mimeType = "image/png"),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    runCurrent()

    assertEquals(1, fixture.reselectFailures)
    assertTrue(fixture.persistence.reserveCalls.isEmpty())
    assertTrue(fixture.released.contains("broken.png"))
  }

  @Test
  fun reselectReportsAFailureWhenTheReservationFails() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("node-a", id = "FILE7")))
    fixture.persistence.onReserve = { _, _ -> throw TypieError("internal_server_error", null) }

    assertTrue(
      fixture.importer.reselect(
        session = fixture.session,
        assetId = "FILE7",
        nodeId = "node-a",
        kind = IncomingContentItem.Kind.File,
        file = pickedFile("b.pdf", fixture.released),
        onFailure = { fixture.reselectFailures += 1 },
      )
    )
    runCurrent()

    assertEquals(1, fixture.reselectFailures)
    assertTrue(fixture.released.contains("b.pdf"))
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  @Test
  fun cancelNodeStopsAnInFlightTransferAndItsHeartbeat() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    val gate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> gate.await() }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()

    fixture.importer.cancelNode("existing")
    runCurrent()

    assertFalse(fixture.importer.canRetry("FILE1"))
    assertTrue(fixture.state.files.uploads.isEmpty())
    assertTrue(fixture.released.contains("a.pdf"))
    advanceTimeBy(AssetHeartbeatIntervalMs * 3)
    runCurrent()
    assertTrue(fixture.heartbeats.isEmpty())

    gate.complete(Unit)
    runCurrent()
    assertTrue(fixture.persistence.finalizeFileCalls.isEmpty())
  }

  @Test
  fun cancelNodeAfterAFailureStopsTheRunFromResurrectingTheCard() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    fixture.persistence.onUpload = { _, _ -> throw IllegalStateException("network down") }

    fixture.importInto("existing", IncomingContentItem.Kind.File, listOf(fixture.fileItem("a.pdf")))
    runCurrent()
    assertNotNull(fixture.state.files.uploads["existing"])

    fixture.importer.cancelNode("existing")
    runCurrent()

    assertTrue(fixture.state.files.uploads.isEmpty())
    assertFalse(fixture.importer.retry("FILE1"))
    assertTrue(fixture.released.contains("a.pdf"))
  }

  @Test
  fun cancellingWhileRecordingKeepsTheSourceOfARunThatAlreadyStarted() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a", "node-b")))
    val transferGate = CompletableDeferred<Unit>()
    fixture.persistence.onUpload = { _, _ -> transferGate.await() }
    var importJob: Job? = null
    var nodeOps = 0
    fixture.document.onNodeOp = {
      nodeOps += 1
      if (nodeOps == 2) importJob?.cancel()
    }

    importJob = backgroundScope.launch {
      fixture.importAtSelection(listOf(fixture.fileItem("a.pdf"), fixture.fileItem("b.pdf")))
    }
    importJob.join()
    runCurrent()

    assertNotNull(fixture.state.files.uploads["node-a"])
    assertFalse(fixture.released.contains("a.pdf"))
    assertTrue(fixture.released.contains("b.pdf"))

    transferGate.complete(Unit)
    runCurrent()
  }

  @Test
  fun cancellingDuringTheSizeProbeStopsReadingInsteadOfDrainingTheWholeSource() = runTest {
    val fixture = Fixture(this, initialElements = listOf(fileElement("existing")))
    var importJob: Job? = null
    val rawSource = CancellingRawSource(totalBytes = 300_000, cancelAtPull = 3) { importJob }
    val source =
      pickedFile("big.bin", fixture.released, size = null, sourceFactory = { rawSource.buffered() })

    importJob = backgroundScope.launch {
      fixture.importInto(
        "existing",
        IncomingContentItem.Kind.File,
        listOf(IncomingContentItem(kind = IncomingContentItem.Kind.File, file = source)),
      )
    }
    importJob.join()
    runCurrent()

    assertEquals(3, rawSource.pulls)
    assertTrue(fixture.persistence.reserveCalls.isEmpty())
  }

  @Test
  fun cancellingDuringTheSizeProbeReleasesEverySourceInTheBatch() = runTest {
    val fixture = Fixture(this, receipts = listOf(listOf("node-a", "node-b")))
    var importJob: Job? = null
    val rawSource = CancellingRawSource(totalBytes = 300_000, cancelAtPull = 3) { importJob }
    val first =
      pickedFile("a.bin", fixture.released, size = null, sourceFactory = { rawSource.buffered() })
    val second = pickedFile("b.bin", fixture.released, size = null, sourceBytes = 8)

    importJob = backgroundScope.launch {
      fixture.importAtSelection(
        listOf(
          IncomingContentItem(kind = IncomingContentItem.Kind.File, file = first),
          IncomingContentItem(kind = IncomingContentItem.Kind.File, file = second),
        )
      )
    }
    importJob.join()
    runCurrent()

    assertEquals(setOf("a.bin", "b.bin"), fixture.released.toSet())
    assertTrue(fixture.persistence.reserveCalls.isEmpty())
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  @Test
  fun keepsAServerSuppliedContentTypeFieldInsteadOfAppendingASecondOne() {
    val source = pickedFile("a.bin", mutableListOf(), mimeType = "image/png")
    val form = blobUploadForm(mapOf("key" to "k", "Content-Type" to "image/png"), source)

    assertEquals(mapOf("key" to "k", "Content-Type" to "image/png"), form.fields)
    assertEquals("image/png", form.filePartContentType)
  }

  private class CancellingRawSource(
    private val totalBytes: Int,
    private val cancelAtPull: Int,
    private val chunkBytes: Int = 8 * 1024,
    private val job: () -> Job?,
  ) : RawSource {
    var pulls = 0
      private set

    private var remaining = totalBytes

    override fun readAtMostTo(sink: Buffer, byteCount: Long): Long {
      if (remaining <= 0) return -1L
      pulls += 1
      if (pulls == cancelAtPull) job()?.cancel()
      val take = minOf(chunkBytes.toLong(), byteCount, remaining.toLong()).toInt()
      sink.write(ByteArray(take))
      remaining -= take
      return take.toLong()
    }

    override fun close() = Unit
  }

  private class FakeAttachmentPersistence : EditorAttachmentPersistence {
    val order = mutableListOf<String>()
    val reserveCalls = mutableListOf<Pair<String, List<EditorAttachmentReserveItem>>>()
    val uploadCalls = mutableListOf<Pair<EditorAttachmentReservation, PickedFile>>()
    val finalizeImageCalls = mutableListOf<Pair<String, String>>()
    val finalizeFileCalls = mutableListOf<Pair<String, String>>()
    val abandonCalls = mutableListOf<Pair<String, String>>()

    var onReserve:
      (suspend (String, List<EditorAttachmentReserveItem>) -> List<EditorAttachmentReservation>)? =
      null
    var onUpload: (suspend (EditorAttachmentReservation, PickedFile) -> Unit)? = null
    var onFinalizeImage: (suspend (String, String) -> EditorImageAsset)? = null
    var onFinalizeFile: (suspend (String, String) -> EditorFileAsset)? = null
    var onAbandon: (suspend (String, String) -> Boolean)? = null

    private var assetSeq = 0
    private var nonceSeq = 0

    fun reservationFor(item: EditorAttachmentReserveItem): EditorAttachmentReservation {
      assetSeq += 1
      nonceSeq += 1
      val assetId =
        item.assetId
          ?: when (item.kind) {
            IncomingContentItem.Kind.Image -> "IMG$assetSeq"
            IncomingContentItem.Kind.File -> "FILE$assetSeq"
          }
      return EditorAttachmentReservation(
        assetId = assetId,
        nonce = "n$nonceSeq",
        path = "26/07/a/ab/$assetId",
        uploadUrl = "https://uploads.example/bucket",
        uploadFields = mapOf("key" to "k", "Content-Type" to item.format),
      )
    }

    fun imageAsset(id: String) =
      EditorImageAsset(
        id = id,
        url = "https://example.com/$id.webp",
        width = 640,
        height = 480,
        ratio = 640.0 / 480.0,
        placeholder = null,
      )

    fun fileAsset(id: String) =
      EditorFileAsset(id = id, name = "$id.pdf", url = "https://example.com/$id.pdf", size = 1024)

    override suspend fun reserve(
      documentId: String,
      items: List<EditorAttachmentReserveItem>,
    ): List<EditorAttachmentReservation> {
      order += "reserve"
      reserveCalls += documentId to items
      return onReserve?.invoke(documentId, items) ?: items.map(::reservationFor)
    }

    override suspend fun upload(
      reservation: EditorAttachmentReservation,
      source: PickedFile,
    ) {
      order += "upload"
      uploadCalls += reservation to source
      onUpload?.invoke(reservation, source)
    }

    override suspend fun finalizeImage(assetId: String, nonce: String): EditorImageAsset {
      order += "finalize"
      finalizeImageCalls += assetId to nonce
      return onFinalizeImage?.invoke(assetId, nonce) ?: imageAsset(assetId)
    }

    override suspend fun finalizeFile(assetId: String, nonce: String): EditorFileAsset {
      order += "finalize"
      finalizeFileCalls += assetId to nonce
      return onFinalizeFile?.invoke(assetId, nonce) ?: fileAsset(assetId)
    }

    override suspend fun abandon(assetId: String, nonce: String): Boolean {
      order += "abandon"
      abandonCalls += assetId to nonce
      return onAbandon?.invoke(assetId, nonce) ?: true
    }
  }

  private class FakeDocument(receipts: List<List<String>>) {
    val elements = mutableListOf<ExternalElement>()
    val flushLog = mutableListOf<String>()
    var onNodeOp: ((NodeOp) -> Unit)? = null
    lateinit var fake: FakeFfiEditor

    private val pendingReceipts = receipts.toMutableList()
    private var processed = 0

    fun tick(): List<EditorEvent> {
      val events = mutableListOf<EditorEvent>()
      var lastKind: String? = null
      while (processed < fake.enqueued.size) {
        when (val message = fake.enqueued[processed++]) {
          is Message.Insertion -> {
            val op = message.op
            if (op is InsertionOp.AttachmentPlaceholders) {
              lastKind = "insertion"
              val nodeIds = if (pendingReceipts.isEmpty()) null else pendingReceipts.removeAt(0)
              if (nodeIds != null) {
                nodeIds.forEachIndexed { index, nodeId ->
                  elements +=
                    when (op.kinds.getOrNull(index)) {
                      AttachmentPlaceholderKind.File -> fileElement(nodeId)
                      else -> imageElement(nodeId)
                    }
                }
                events +=
                  EditorEvent.AttachmentPlaceholdersInserted(
                    requestId = op.requestId,
                    nodeIds = nodeIds,
                  )
              }
            }
          }
          is Message.Node -> {
            lastKind = "node"
            apply(message.op)
            onNodeOp?.invoke(message.op)
          }
          else -> Unit
        }
      }
      if (lastKind != null) flushLog += lastKind
      return events
    }

    private fun apply(op: NodeOp) {
      when (op) {
        is NodeOp.SetAttr -> {
          val attr = op.attr
          if (attr is NodeAttr.Image) {
            val value = attr.attr
            if (value is ImageNodeAttr.Id) replace(op.id) { imageElement(op.id, value.value) }
          }
        }
        is NodeOp.SetAttrs -> {
          val attrs = op.attrs
          if (attrs is PlainNode.File) replace(op.id) { fileElement(op.id, attrs.id) }
        }
        is NodeOp.Delete -> elements.removeAll { it.node == op.id }
        else -> Unit
      }
    }

    private fun replace(nodeId: String, build: () -> ExternalElement) {
      val index = elements.indexOfFirst { it.node == nodeId }
      if (index >= 0) elements[index] = build()
    }
  }

  private class Fixture(
    testScope: TestScope,
    initialElements: List<ExternalElement> = emptyList(),
    receipts: List<List<String>> = emptyList(),
    finalizeRetryDelaysMs: List<Long> = emptyList(),
    settleAttempts: Int = 3,
    maxReserveItems: Int = 50,
  ) {
    val state = EditorExternalElementState()
    val persistence = FakeAttachmentPersistence()
    val document = FakeDocument(receipts)
    val released = mutableListOf<String>()
    val heartbeats = mutableListOf<Pair<String, List<WsAssetItem>>>()
    val assetFailed = mutableListOf<Pair<String, List<WsAssetItem>>>()
    val refreshed = mutableListOf<List<String>>()
    var reselectFailures = 0

    var sessionCurrent = true
    var onRefresh: ((List<String>) -> Unit)? = null

    val backgroundJob = Job(testScope.backgroundScope.coroutineContext[Job])

    val fake =
      FakeFfiEditor(
        onTick = { document.tick() },
        externalElementsProvider = { document.elements.toList() },
      )
    val editor: Editor
    val session: DocumentEditingSession
    val importer: DefaultEditorAttachmentImporter

    init {
      document.fake = fake
      document.elements += initialElements
      editor = Editor(fake, testScope, StandardTestDispatcher(testScope.testScheduler))
      editor.sync {}
      session = createTestDocumentEditingSession(editor, testScope)
      importer =
        DefaultEditorAttachmentImporter(
          externalElementState = state,
          bringIntoViewRequests = EditorBringIntoViewRequests(),
          isSessionCurrent = { sessionCurrent && it === session },
          backgroundScope =
            CoroutineScope(testScope.backgroundScope.coroutineContext + backgroundJob),
          sendAssetHeartbeat = { documentId, items -> heartbeats += documentId to items },
          sendAssetFailed = { documentId, items -> assetFailed += documentId to items },
          refreshAssets = { ids ->
            refreshed += ids
            onRefresh?.invoke(ids)
          },
          persistence = persistence,
          finalizeRetryDelaysMs = finalizeRetryDelaysMs,
          settleDelayMs = 500,
          settleAttempts = settleAttempts,
          maxReserveItems = maxReserveItems,
        )
    }

    suspend fun importAtSelection(items: List<IncomingContentItem>): Int {
      var imported = -1
      importer.import(
        session = session,
        items = items,
        destination = EditorAttachmentDestination.CurrentSelection,
        onCompleted = { imported = it },
      )
      return imported
    }

    suspend fun importInto(
      nodeId: String,
      kind: IncomingContentItem.Kind,
      items: List<IncomingContentItem>,
    ): Int {
      var imported = -1
      importer.import(
        session = session,
        items = items,
        destination =
          EditorAttachmentDestination.ExistingPlaceholder(nodeId = nodeId, expectedKind = kind),
        onCompleted = { imported = it },
      )
      return imported
    }

    fun nodeOps(): List<NodeOp> =
      fake.enqueued.filterIsInstance<Message.Node>().map(Message.Node::op)

    fun placeholderRequests(): List<InsertionOp.AttachmentPlaceholders> =
      fake.enqueued
        .filterIsInstance<Message.Insertion>()
        .map(Message.Insertion::op)
        .filterIsInstance<InsertionOp.AttachmentPlaceholders>()

    fun committedAssetIdsByNode(): Map<String, String?> =
      nodeOps().associate { op ->
        when (op) {
          is NodeOp.SetAttr -> op.id to ((op.attr as NodeAttr.Image).attr as ImageNodeAttr.Id).value
          is NodeOp.SetAttrs -> op.id to (op.attrs as PlainNode.File).id
          else -> error("Unexpected node operation: $op")
        }
      }

    fun imageItem(
      filename: String,
      mimeType: String? = "image/png",
      width: Int? = 10,
    ): IncomingContentItem =
      IncomingContentItem(
        kind = IncomingContentItem.Kind.Image,
        file =
          pickedFile(
            filename = filename,
            released = released,
            mimeType = mimeType,
            width = width,
            height = if (width == null) null else 5,
          ),
      )

    fun fileItem(filename: String): IncomingContentItem =
      IncomingContentItem(
        kind = IncomingContentItem.Kind.File,
        file = pickedFile(filename = filename, released = released),
      )
  }

  private companion object {
    fun imageElement(nodeId: String, id: String? = null): ExternalElement =
      ExternalElement(
        pageIdx = 0,
        node = nodeId,
        bounds = Rect(0f, 0f, 1f, 1f),
        isSelected = false,
        data = ExternalElementData.Image(id = id, proportion = 100),
      )

    fun fileElement(nodeId: String, id: String? = null): ExternalElement =
      ExternalElement(
        pageIdx = 0,
        node = nodeId,
        bounds = Rect(0f, 0f, 1f, 1f),
        isSelected = false,
        data = ExternalElementData.File(id = id),
      )

    fun pickedFile(
      filename: String,
      released: MutableList<String>,
      mimeType: String? = "application/pdf",
      size: Long? = 1,
      width: Int? = null,
      height: Int? = null,
      sourceBytes: Int? = 0,
      sourceFactory: (() -> Source)? = null,
    ): PickedFile =
      PickedFile(
        filename = filename,
        mimeType = mimeType,
        size = size,
        previewModel = Unit,
        imageWidth = width,
        imageHeight = height,
        openSource = {
          sourceFactory?.invoke()
            ?: run {
              if (sourceBytes == null) error("source unavailable")
              Buffer().apply { write(ByteArray(sourceBytes)) }
            }
        },
        release = { released += filename },
      )
  }
}
