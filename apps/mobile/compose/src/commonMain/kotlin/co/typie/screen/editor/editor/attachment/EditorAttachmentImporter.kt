@file:OptIn(ExperimentalUuidApi::class)

package co.typie.screen.editor.editor.attachment

import androidx.compose.runtime.compositionLocalOf
import co.typie.domain.blob.normalizeBlobContentType
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.external.EditorAssetResolution
import co.typie.editor.external.EditorAttachmentFailureStage
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorFileUpload
import co.typie.editor.external.EditorImageUpload
import co.typie.editor.ffi.AttachmentPlaceholderKind
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.ImageNodeAttr
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeAttr
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.sync.ws.WsAssetItem
import co.typie.graphql.TypieError
import co.typie.platform.IncomingContentItem
import co.typie.platform.PickedFile
import co.typie.screen.editor.editor.toolbar.contextual.AttachmentKind
import co.typie.screen.editor.editor.toolbar.contextual.reportAttachmentFailure
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import kotlinx.coroutines.withContext

internal sealed interface EditorAttachmentDestination {
  data object CurrentSelection : EditorAttachmentDestination

  data class ExistingPlaceholder(val nodeId: String, val expectedKind: IncomingContentItem.Kind) :
    EditorAttachmentDestination
}

internal interface EditorAttachmentImporter {
  suspend fun import(
    session: DocumentEditingSession,
    items: List<IncomingContentItem>,
    destination: EditorAttachmentDestination,
    onCompleted: (importedCount: Int) -> Unit,
  ): Boolean

  fun retry(assetId: String): Boolean

  fun canRetry(assetId: String): Boolean

  fun reselect(
    session: DocumentEditingSession,
    assetId: String,
    nodeId: String,
    kind: IncomingContentItem.Kind,
    file: PickedFile,
    onFailure: () -> Unit,
  ): Boolean

  fun retireCompleted(assetIds: Collection<String>)

  fun cancelNode(nodeId: String)

  fun setForeground(foreground: Boolean)

  fun dispose()
}

internal val LocalEditorAttachmentImporter =
  compositionLocalOf<EditorAttachmentImporter> { error("No EditorAttachmentImporter provided") }

internal fun interface EditorAttachmentPicker {
  fun pick(kind: IncomingContentItem.Kind, nodeId: String, reclaimAssetId: String?)
}

internal data class EditorAttachmentPickRequest(
  val session: DocumentEditingSession,
  val nodeId: String,
  val reclaimAssetId: String?,
)

internal val LocalEditorAttachmentPicker =
  compositionLocalOf<EditorAttachmentPicker> { error("No EditorAttachmentPicker provided") }

internal const val AssetHeartbeatIntervalMs = 20_000L

private val FinalizeRetryDelaysMs = listOf(500L, 2_000L, 5_000L)
private const val SettleDelayMs = 500L
private const val SettleAttempts = 3
private const val MaxReserveItems = 50
private const val MaxConcurrentTransfers = 5
private const val MaxAssetItemsPerMessage = 100
private const val SizeProbeChunkBytes = 64 * 1024

private enum class FinalizeOutcome {
  Done,
  Failed,
  Expired,
  Cancelled,
}

private enum class SettleOutcome {
  Reserve,
  Ready,
  Cancelled,
}

private enum class AssetProbe {
  Ready,
  Pending,
  Missing,
  Unknown,
}

private sealed interface ReclaimOutcome {
  data class Reserved(val reservation: EditorAttachmentReservation) : ReclaimOutcome

  data object AlreadyCompleted : ReclaimOutcome

  data object Failed : ReclaimOutcome
}

internal class DefaultEditorAttachmentImporter(
  private val externalElementState: EditorExternalElementState,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val isSessionCurrent: (DocumentEditingSession) -> Boolean,
  private val backgroundScope: CoroutineScope,
  private val sendAssetHeartbeat: (documentId: String, items: List<WsAssetItem>) -> Unit,
  private val sendAssetFailed: (documentId: String, items: List<WsAssetItem>) -> Unit,
  private val refreshAssets: (ids: List<String>) -> Unit,
  private val persistence: EditorAttachmentPersistence = GraphqlEditorAttachmentPersistence,
  private val heartbeatIntervalMs: Long = AssetHeartbeatIntervalMs,
  private val finalizeRetryDelaysMs: List<Long> = FinalizeRetryDelaysMs,
  private val settleDelayMs: Long = SettleDelayMs,
  private val settleAttempts: Int = SettleAttempts,
  private val maxReserveItems: Int = MaxReserveItems,
  private val maxConcurrentTransfers: Int = MaxConcurrentTransfers,
) : EditorAttachmentImporter {
  private class PreparedItem(
    val item: IncomingContentItem,
    val previewModel: Any?,
    val width: Int,
    val height: Int,
    val size: Long,
  )

  private class UploadRun(
    val assetId: String,
    val session: DocumentEditingSession,
    val editor: Editor,
    val nodeId: String,
    val prepared: PreparedItem,
    var nonce: String,
  ) {
    val kind: IncomingContentItem.Kind
      get() = prepared.item.kind

    val documentId: String
      get() = session.documentId

    var cancelled = false
    var failedStage: EditorAttachmentFailureStage? = null
    var job: Job? = null
    var pending: Any? = null
  }

  private val sources = EditorAttachmentSourceStore()
  private val runs = mutableMapOf<String, UploadRun>()
  private val claimedNodes = mutableSetOf<String>()
  private val transferLimit = Semaphore(maxConcurrentTransfers)
  private var heartbeatJob: Job? = null
  private var foreground = true
  private var disposed = false

  override suspend fun import(
    session: DocumentEditingSession,
    items: List<IncomingContentItem>,
    destination: EditorAttachmentDestination,
    onCompleted: (importedCount: Int) -> Unit,
  ): Boolean {
    if (items.isEmpty()) {
      onCompleted(0)
      return false
    }
    if (disposed || !isSessionCurrent(session)) {
      items.closeAll()
      onCompleted(0)
      return false
    }

    val editor = session.editor
    val prepared = mutableListOf<PreparedItem>()
    try {
      for (item in items) prepare(item)?.let(prepared::add)
    } catch (error: Throwable) {
      items.closeAll()
      throw error
    }
    items.filterNot { item -> prepared.any { it.item === item } }.closeAll()
    if (prepared.isEmpty()) {
      onCompleted(0)
      return false
    }

    val existingNodeId =
      when (destination) {
        is EditorAttachmentDestination.CurrentSelection -> null
        is EditorAttachmentDestination.ExistingPlaceholder -> {
          val first = prepared.first()
          if (
            first.item.kind != destination.expectedKind ||
              !isAvailable(editor, destination.nodeId, destination.expectedKind)
          ) {
            prepared.closeSources()
            onCompleted(0)
            return false
          }
          destination.nodeId
        }
      }

    claim(existingNodeId)
    val importedCount =
      try {
        reserveThenRecord(session, editor, prepared, existingNodeId)
      } catch (error: Throwable) {
        prepared.closeUnownedSources()
        throw error
      } finally {
        release(existingNodeId)
      }
    onCompleted(importedCount)
    return importedCount > 0
  }

  override fun retry(assetId: String): Boolean {
    val run = runs[assetId] ?: return false
    val stage = run.failedStage ?: return false
    if (stage == EditorAttachmentFailureStage.Transfer && sources.get(assetId) == null) return false
    markActive(run)
    schedule(run) { restart(run, stage) }
    return true
  }

  override fun canRetry(assetId: String): Boolean {
    val run = runs[assetId] ?: return false
    val stage = run.failedStage ?: return false
    return stage != EditorAttachmentFailureStage.Transfer || sources.get(assetId) != null
  }

  override fun reselect(
    session: DocumentEditingSession,
    assetId: String,
    nodeId: String,
    kind: IncomingContentItem.Kind,
    file: PickedFile,
    onFailure: () -> Unit,
  ): Boolean {
    if (disposed || !isSessionCurrent(session)) {
      file.close()
      return false
    }
    val item = IncomingContentItem(kind = kind, file = file)
    backgroundScope
      .launch { runReselect(session, assetId, nodeId, item, onFailure) }
      .invokeOnCompletion { cause -> if (cause != null) file.close() }
    return true
  }

  override fun retireCompleted(assetIds: Collection<String>) {
    assetIds.forEach { assetId -> runs[assetId]?.let(::retire) }
  }

  override fun cancelNode(nodeId: String) {
    runs.values.filter { it.nodeId == nodeId }.forEach(::retire)
    externalElementState.images.uploads.remove(nodeId)
    externalElementState.files.uploads.remove(nodeId)
  }

  override fun setForeground(foreground: Boolean) {
    this.foreground = foreground
    if (foreground) {
      failDeadTransfers()
      ensureHeartbeat()
    } else {
      stopHeartbeat()
    }
  }

  override fun dispose() {
    disposed = true
    runs.values.toList().forEach(::retire)
    runs.clear()
    sources.clear()
    stopHeartbeat()
  }

  private suspend fun reserveThenRecord(
    session: DocumentEditingSession,
    editor: Editor,
    prepared: List<PreparedItem>,
    existingNodeId: String?,
  ): Int {
    val reservations = reserve(session.documentId, prepared.map { entry -> toReserveItem(entry) })
    if (reservations == null) {
      prepared.closeSources()
      return 0
    }

    val head = if (existingNodeId == null) null else prepared.first()
    val tail = if (head == null) prepared else prepared.drop(1)

    if (
      !isSessionCurrent(session) ||
        (existingNodeId != null &&
          head != null &&
          !isWritable(editor, existingNodeId, head.item.kind))
    ) {
      prepared.closeSources()
      return 0
    }

    val targets = mutableListOf<Triple<String, PreparedItem, EditorAttachmentReservation>>()
    if (head != null && existingNodeId != null) {
      targets += Triple(existingNodeId, head, reservations.first())
    }

    if (tail.isNotEmpty()) {
      val nodeIds = insertPlaceholders(session, editor, tail)
      if (
        nodeIds == null ||
          nodeIds.size != tail.size ||
          nodeIds.toSet().size != nodeIds.size ||
          (existingNodeId != null && existingNodeId in nodeIds)
      ) {
        prepared.closeSources()
        return 0
      }
      val offset = if (head == null) 0 else 1
      tail.forEachIndexed { index, entry ->
        targets += Triple(nodeIds[index], entry, reservations[index + offset])
      }
    }

    var recorded = 0
    for ((nodeId, entry, reservation) in targets) {
      if (!writeAssetId(session, editor, nodeId, entry.item.kind, reservation.assetId)) {
        entry.item.file.close()
        continue
      }
      startRun(session, editor, nodeId, entry, reservation)
      recorded += 1
    }
    return recorded
  }

  /**
   * 이미 완결된 ID를 다시 잡으려 하면 서버가 `asset_already_exists`로 막는다 — 실패가 아니라 "그 사이에 해석됐다"는 뜻이므로, 실패 카드 대신 그
   * ID를 pull해 완성 자산을 보여 준다.
   */
  private suspend fun reclaim(
    documentId: String,
    item: EditorAttachmentReserveItem,
    assetId: String,
  ): ReclaimOutcome {
    val reservation =
      try {
        persistence.reserve(documentId, listOf(item)).firstOrNull()
      } catch (error: CancellationException) {
        throw error
      } catch (error: Throwable) {
        if ((error as? TypieError)?.code == "asset_already_exists") {
          refreshAssets(listOf(assetId))
          return ReclaimOutcome.AlreadyCompleted
        }
        reportAttachmentFailure(item.kind.toAttachmentKind(), error)
        null
      }
    return reservation?.let(ReclaimOutcome::Reserved) ?: ReclaimOutcome.Failed
  }

  private suspend fun reserve(
    documentId: String,
    items: List<EditorAttachmentReserveItem>,
  ): List<EditorAttachmentReservation>? =
    try {
      val reservations = coroutineScope {
        items
          .chunked(maxReserveItems)
          .map { chunk -> async { persistence.reserve(documentId, chunk) } }
          .awaitAll()
      }
        .flatten()
      reservations.takeIf { it.size == items.size }
    } catch (error: CancellationException) {
      throw error
    } catch (error: Throwable) {
      reportAttachmentFailure(
        items.firstOrNull()?.kind?.toAttachmentKind() ?: AttachmentKind.File,
        error,
      )
      null
    }

  private suspend fun insertPlaceholders(
    session: DocumentEditingSession,
    editor: Editor,
    tail: List<PreparedItem>,
  ): List<String>? {
    val requestId = Uuid.random().toHexString()
    val kinds = tail.map { it.item.kind.toPlaceholderKind() }
    val command =
      session.submit { _, context ->
        editor.scope.async(context) {
          editor.await(
            beforeCommit = { state ->
              bringIntoViewRequests.requestForVersion(
                target = EditorBringIntoViewTarget.CurrentSelectionHead,
                version = state.version,
              )
            },
            admit = { isSessionCurrent(session) },
            mapEvents = { events ->
              val matches =
                events.filterIsInstance<EditorEvent.AttachmentPlaceholdersInserted>().filter {
                  it.requestId == requestId
                }
              if (matches.size == 1) matches.single().nodeIds else emptyList()
            },
          ) {
            if (editor.ime?.composing != null) {
              enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
            }
            enqueue(Message.Insertion(InsertionOp.AttachmentPlaceholders(requestId, kinds)))
          }
        }
      } ?: return null
    return command.await()
  }

  private suspend fun writeAssetId(
    session: DocumentEditingSession,
    editor: Editor,
    nodeId: String,
    kind: IncomingContentItem.Kind,
    assetId: String,
  ): Boolean {
    val command =
      session.submit { _, context ->
        editor.scope.async(context) {
          editor.await(
            beforeCommit = { state ->
              bringIntoViewRequests.requestForVersion(
                target = EditorBringIntoViewTarget.CurrentSelectionHead,
                version = state.version,
              )
            },
            admit = { isSessionCurrent(session) && isEmptyPlaceholder(editor, nodeId, kind) },
          ) {
            if (editor.ime?.composing != null) {
              enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
            }
            enqueue(
              when (kind) {
                IncomingContentItem.Kind.Image ->
                  Message.Node(
                    NodeOp.SetAttr(id = nodeId, attr = NodeAttr.Image(ImageNodeAttr.Id(assetId)))
                  )
                IncomingContentItem.Kind.File ->
                  Message.Node(NodeOp.SetAttrs(id = nodeId, attrs = PlainNode.File(id = assetId)))
              }
            )
          }
        }
      } ?: return false
    return command.await()
  }

  private fun startRun(
    session: DocumentEditingSession,
    editor: Editor,
    nodeId: String,
    prepared: PreparedItem,
    reservation: EditorAttachmentReservation,
  ) {
    val run =
      UploadRun(
        assetId = reservation.assetId,
        session = session,
        editor = editor,
        nodeId = nodeId,
        prepared = prepared,
        nonce = reservation.nonce,
      )
    runs[run.assetId] = run
    sources.put(run.assetId, prepared.item.file)
    markActive(run)
    schedule(run) { runPipeline(run, reservation) }
  }

  private fun schedule(run: UploadRun, block: suspend () -> Unit) {
    run.job = backgroundScope.launch { transferLimit.withPermit { block() } }
  }

  private suspend fun runPipeline(run: UploadRun, reservation: EditorAttachmentReservation?) {
    var current = reservation
    var reReserved = false

    while (true) {
      if (current != null) {
        val source = sources.get(run.assetId)
        if (source == null) {
          fail(run, EditorAttachmentFailureStage.Transfer, null)
          return
        }
        try {
          persistence.upload(current, source)
        } catch (error: CancellationException) {
          throw error
        } catch (error: Throwable) {
          if (run.cancelled) return
          sendAssetFailed(run.documentId, listOf(WsAssetItem(id = run.assetId, nonce = run.nonce)))
          fail(run, EditorAttachmentFailureStage.Transfer, error)
          return
        }
        if (run.cancelled) return
      }

      if (finalizeLoop(run) != FinalizeOutcome.Expired) return

      if (reReserved) {
        fail(run, EditorAttachmentFailureStage.Transfer, null)
        return
      }
      reReserved = true

      if (!settleRun(run)) return
      current = reReserve(run) ?: return
    }
  }

  private suspend fun finalizeLoop(run: UploadRun): FinalizeOutcome {
    var attempt = 0
    while (true) {
      try {
        when (run.kind) {
          IncomingContentItem.Kind.Image -> {
            val asset = persistence.finalizeImage(run.assetId, run.nonce)
            if (run.cancelled) return FinalizeOutcome.Cancelled
            externalElementState.putResolved(asset)
          }
          IncomingContentItem.Kind.File -> {
            val asset = persistence.finalizeFile(run.assetId, run.nonce)
            if (run.cancelled) return FinalizeOutcome.Cancelled
            externalElementState.putResolved(asset)
          }
        }
        complete(run)
        return FinalizeOutcome.Done
      } catch (error: CancellationException) {
        throw error
      } catch (error: Throwable) {
        if (run.cancelled) return FinalizeOutcome.Cancelled
        when ((error as? TypieError)?.code) {
          "asset_finalize_rejected",
          "forbidden",
          "validation_error" -> {
            fail(run, EditorAttachmentFailureStage.FinalizeRejected, error)
            return FinalizeOutcome.Failed
          }
          "asset_upload_expired" -> return FinalizeOutcome.Expired
        }
        val delayMs = finalizeRetryDelaysMs.getOrNull(attempt)
        if (delayMs == null) {
          fail(run, EditorAttachmentFailureStage.FinalizeRetryable, error)
          return FinalizeOutcome.Failed
        }
        attempt += 1
        delay(delayMs)
        if (run.cancelled) return FinalizeOutcome.Cancelled
      }
    }
  }

  private suspend fun restart(run: UploadRun, stage: EditorAttachmentFailureStage) {
    if (stage != EditorAttachmentFailureStage.Transfer) {
      if (finalizeLoop(run) != FinalizeOutcome.Expired) return
    }
    if (!settleRun(run)) return
    val renewed = reReserve(run) ?: return
    runPipeline(run, renewed)
  }

  private suspend fun runReselect(
    session: DocumentEditingSession,
    assetId: String,
    nodeId: String,
    item: IncomingContentItem,
    onFailure: () -> Unit,
  ) {
    val prepared = prepare(item)
    if (prepared == null) {
      item.file.close()
      onFailure()
      return
    }

    val previous = runs[assetId]
    if (previous != null) {
      previous.cancelled = true
      previous.job?.cancel()
      val outcome = settle(assetId, previous.nonce, previous.kind) { false }
      retire(previous)
      if (outcome == SettleOutcome.Ready) {
        prepared.item.file.close()
        return
      }
    }

    if (disposed || !isSessionCurrent(session)) {
      prepared.item.file.close()
      return
    }

    when (val outcome = reclaim(session.documentId, toReserveItem(prepared, assetId), assetId)) {
      is ReclaimOutcome.Reserved ->
        startRun(session, session.editor, nodeId, prepared, outcome.reservation)
      // 이미 완결된 자산이 렌더되므로 사용자가 고른 파일은 버린다(실패 아님).
      ReclaimOutcome.AlreadyCompleted -> prepared.item.file.close()
      ReclaimOutcome.Failed -> {
        prepared.item.file.close()
        onFailure()
      }
    }
  }

  private suspend fun settleRun(run: UploadRun): Boolean {
    val outcome = settle(run.assetId, run.nonce, run.kind) { run.cancelled }
    if (outcome == SettleOutcome.Ready) retire(run)
    return outcome == SettleOutcome.Reserve
  }

  private suspend fun settle(
    assetId: String,
    nonce: String,
    kind: IncomingContentItem.Kind,
    isCancelled: () -> Boolean,
  ): SettleOutcome {
    repeat(settleAttempts) {
      val abandoned =
        try {
          persistence.abandon(assetId, nonce)
        } catch (error: CancellationException) {
          throw error
        } catch (_: Throwable) {
          false
        }
      if (isCancelled()) return SettleOutcome.Cancelled
      if (abandoned) return SettleOutcome.Reserve

      refreshAssets(listOf(assetId))
      delay(settleDelayMs)
      if (isCancelled()) return SettleOutcome.Cancelled

      when (probe(assetId, kind)) {
        AssetProbe.Ready -> return SettleOutcome.Ready
        AssetProbe.Missing -> return SettleOutcome.Reserve
        else -> Unit
      }
    }
    return SettleOutcome.Reserve
  }

  private suspend fun reReserve(run: UploadRun): EditorAttachmentReservation? {
    val outcome = reclaim(run.documentId, toReserveItem(run.prepared, run.assetId), run.assetId)
    if (outcome is ReclaimOutcome.AlreadyCompleted) {
      // 그 사이에 이 ID가 완결됐다 — 실패 카드 대신 완성 자산을 보여 준다.
      retire(run)
      return null
    }
    if (outcome !is ReclaimOutcome.Reserved) {
      fail(run, EditorAttachmentFailureStage.Transfer, null)
      return null
    }
    if (run.cancelled) return null
    run.nonce = outcome.reservation.nonce
    markActive(run)
    return outcome.reservation
  }

  private fun probe(assetId: String, kind: IncomingContentItem.Kind): AssetProbe {
    val hasAsset =
      when (kind) {
        IncomingContentItem.Kind.Image -> externalElementState.images.assets.containsKey(assetId)
        IncomingContentItem.Kind.File -> externalElementState.files.assets.containsKey(assetId)
      }
    if (hasAsset) return AssetProbe.Ready
    return when (externalElementState.resolutions[assetId]) {
      is EditorAssetResolution.Pending -> AssetProbe.Pending
      EditorAssetResolution.Missing -> AssetProbe.Missing
      else -> AssetProbe.Unknown
    }
  }

  private fun markActive(run: UploadRun) {
    run.failedStage = null
    writeUploadState(run, null)
    ensureHeartbeat()
  }

  private fun fail(
    run: UploadRun,
    stage: EditorAttachmentFailureStage,
    error: Throwable?,
  ) {
    if (run.cancelled) return
    run.failedStage = stage
    writeUploadState(run, stage)
    stopHeartbeatIfIdle()
    if (error != null) reportAttachmentFailure(run.kind.toAttachmentKind(), error)
  }

  private fun writeUploadState(run: UploadRun, stage: EditorAttachmentFailureStage?) {
    when (run.kind) {
      IncomingContentItem.Kind.Image -> {
        val upload =
          EditorImageUpload(
            previewModel = run.prepared.previewModel ?: Unit,
            name = run.prepared.item.file.filename,
            width = run.prepared.width,
            height = run.prepared.height,
            failedStage = stage,
          )
        run.pending = upload
        externalElementState.images.uploads[run.nodeId] = upload
      }
      IncomingContentItem.Kind.File -> {
        val upload =
          EditorFileUpload(
            name = run.prepared.item.file.filename,
            size = run.prepared.size,
            failedStage = stage,
          )
        run.pending = upload
        externalElementState.files.uploads[run.nodeId] = upload
      }
    }
  }

  private fun retire(run: UploadRun) = finish(run, cancelJob = true)

  private fun complete(run: UploadRun) = finish(run, cancelJob = false)

  private fun finish(run: UploadRun, cancelJob: Boolean) {
    run.cancelled = true
    if (cancelJob) run.job?.cancel()
    if (runs[run.assetId] === run) runs.remove(run.assetId)
    when (run.kind) {
      IncomingContentItem.Kind.Image ->
        if (externalElementState.images.uploads[run.nodeId] === run.pending) {
          externalElementState.images.uploads.remove(run.nodeId)
        }
      IncomingContentItem.Kind.File ->
        if (externalElementState.files.uploads[run.nodeId] === run.pending) {
          externalElementState.files.uploads.remove(run.nodeId)
        }
    }
    sources.release(run.assetId)
    stopHeartbeatIfIdle()
  }

  private fun failDeadTransfers() {
    runs.values.toList().forEach { run ->
      if (!run.cancelled && run.failedStage == null && run.job?.isActive == false) {
        fail(run, EditorAttachmentFailureStage.Transfer, null)
      }
    }
  }

  private fun activeItems(): Map<String, List<WsAssetItem>> {
    val grouped = mutableMapOf<String, MutableList<WsAssetItem>>()
    runs.values.forEach { run ->
      if (run.cancelled || run.failedStage != null) return@forEach
      grouped
        .getOrPut(run.documentId) { mutableListOf() }
        .add(WsAssetItem(id = run.assetId, nonce = run.nonce))
    }
    return grouped
  }

  private fun ensureHeartbeat() {
    if (disposed || !foreground || heartbeatJob != null) return
    if (activeItems().isEmpty()) return
    val job = backgroundScope.launch {
      while (true) {
        delay(heartbeatIntervalMs)
        val grouped = activeItems()
        if (grouped.isEmpty()) break
        grouped.forEach { (documentId, items) ->
          items.chunked(MaxAssetItemsPerMessage).forEach { chunk ->
            sendAssetHeartbeat(documentId, chunk)
          }
        }
      }
    }
    heartbeatJob = job
    job.invokeOnCompletion { if (heartbeatJob === job) heartbeatJob = null }
  }

  private fun stopHeartbeat() {
    heartbeatJob?.cancel()
    heartbeatJob = null
  }

  private fun stopHeartbeatIfIdle() {
    if (activeItems().isEmpty()) stopHeartbeat()
  }

  private suspend fun prepare(item: IncomingContentItem): PreparedItem? {
    val width = item.file.imageWidth
    val height = item.file.imageHeight
    if (item.kind == IncomingContentItem.Kind.Image) {
      if (width == null || height == null || width <= 0 || height <= 0) return null
    }
    val size = resolveSize(item.file, item.kind) ?: return null
    return PreparedItem(
      item = item,
      previewModel =
        if (item.kind == IncomingContentItem.Kind.Image) item.file.previewModel else null,
      width = width ?: 0,
      height = height ?: 0,
      size = size,
    )
  }

  private suspend fun resolveSize(file: PickedFile, kind: IncomingContentItem.Kind): Long? =
    file.size
      ?: try {
        withContext(Dispatchers.IO) { measureSourceSize(file) }
      } catch (error: CancellationException) {
        throw error
      } catch (error: Throwable) {
        reportAttachmentFailure(kind.toAttachmentKind(), error)
        null
      }

  private suspend fun measureSourceSize(file: PickedFile): Long {
    var total = 0L
    val buffer = ByteArray(SizeProbeChunkBytes)
    val source = file.openSource()
    try {
      while (true) {
        currentCoroutineContext().ensureActive()
        val read = source.readAtMostTo(buffer, 0, buffer.size)
        if (read < 0) break
        total += read
      }
    } finally {
      source.close()
    }
    return total
  }

  private fun toReserveItem(prepared: PreparedItem, assetId: String? = null) =
    EditorAttachmentReserveItem(
      kind = prepared.item.kind,
      name = prepared.item.file.filename,
      format = normalizeBlobContentType(prepared.item.file.mimeType),
      size = prepared.size,
      assetId = assetId,
    )

  private fun claim(nodeId: String?) {
    if (nodeId != null) claimedNodes.add(nodeId)
  }

  private fun release(nodeId: String?) {
    if (nodeId != null) claimedNodes.remove(nodeId)
  }

  private fun isAvailable(
    editor: Editor,
    nodeId: String,
    kind: IncomingContentItem.Kind,
  ): Boolean = nodeId !in claimedNodes && isWritable(editor, nodeId, kind)

  private fun isWritable(editor: Editor, nodeId: String, kind: IncomingContentItem.Kind): Boolean =
    isEmptyPlaceholder(editor, nodeId, kind) && runs.values.none { it.nodeId == nodeId }

  private fun isEmptyPlaceholder(
    editor: Editor,
    nodeId: String,
    kind: IncomingContentItem.Kind,
  ): Boolean {
    val data = editor.externalElements.firstOrNull { it.node == nodeId }?.data ?: return false
    return when (kind) {
      IncomingContentItem.Kind.Image -> data is ExternalElementData.Image && data.id == null
      IncomingContentItem.Kind.File -> data is ExternalElementData.File && data.id == null
    }
  }

  private fun List<PreparedItem>.closeSources() {
    forEach { it.item.file.close() }
  }

  private fun List<PreparedItem>.closeUnownedSources() {
    forEach { if (!sources.owns(it.item.file)) it.item.file.close() }
  }
}

internal class EditorAttachmentSourceStore {
  private val sources = mutableMapOf<String, PickedFile>()

  fun put(assetId: String, file: PickedFile) {
    val previous = sources.put(assetId, file)
    if (previous !== null && previous !== file) previous.close()
  }

  fun get(assetId: String): PickedFile? = sources[assetId]

  fun owns(file: PickedFile): Boolean = sources.values.any { it === file }

  fun release(assetId: String) {
    sources.remove(assetId)?.close()
  }

  fun clear() {
    sources.values.toList().forEach(PickedFile::close)
    sources.clear()
  }
}

private fun IncomingContentItem.Kind.toPlaceholderKind(): AttachmentPlaceholderKind =
  when (this) {
    IncomingContentItem.Kind.Image -> AttachmentPlaceholderKind.Image
    IncomingContentItem.Kind.File -> AttachmentPlaceholderKind.File
  }

private fun IncomingContentItem.Kind.toAttachmentKind(): AttachmentKind =
  when (this) {
    IncomingContentItem.Kind.Image -> AttachmentKind.Image
    IncomingContentItem.Kind.File -> AttachmentKind.File
  }

private fun List<IncomingContentItem>.closeAll() {
  forEach { it.file.close() }
}
