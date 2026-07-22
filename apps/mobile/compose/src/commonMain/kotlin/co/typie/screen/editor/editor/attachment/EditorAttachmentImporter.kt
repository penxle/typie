@file:OptIn(ExperimentalUuidApi::class)

package co.typie.screen.editor.editor.attachment

import androidx.compose.runtime.compositionLocalOf
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.external.EditorExternalAsset
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
import co.typie.platform.IncomingContentItem
import co.typie.screen.editor.editor.toolbar.contextual.AttachmentKind
import co.typie.screen.editor.editor.toolbar.contextual.completeAttachmentOperation
import co.typie.screen.editor.editor.toolbar.contextual.reportAttachmentFailure
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.launch
import kotlinx.coroutines.supervisorScope
import kotlinx.coroutines.yield

internal sealed interface EditorAttachmentDestination {
  data object CurrentSelection : EditorAttachmentDestination

  data class ExistingPlaceholder(val nodeId: String, val expectedKind: IncomingContentItem.Kind) :
    EditorAttachmentDestination
}

internal fun interface EditorAttachmentImporter {
  suspend fun import(
    session: DocumentEditingSession,
    items: List<IncomingContentItem>,
    destination: EditorAttachmentDestination,
    onCompleted: (importedCount: Int) -> Unit,
  ): Boolean
}

internal val LocalEditorAttachmentImporter =
  compositionLocalOf<EditorAttachmentImporter> { error("No EditorAttachmentImporter provided") }

internal class DefaultEditorAttachmentImporter(
  private val externalElementState: EditorExternalElementState,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val persistence: EditorAttachmentPersistence = GraphqlEditorAttachmentPersistence,
  private val isSessionCurrent: (DocumentEditingSession) -> Boolean,
  private val backgroundScope: CoroutineScope,
) : EditorAttachmentImporter {
  private data class PreparedItem(val item: IncomingContentItem, val pending: Any)

  private data class Target(val nodeId: String, val item: IncomingContentItem, val pending: Any)

  override suspend fun import(
    session: DocumentEditingSession,
    items: List<IncomingContentItem>,
    destination: EditorAttachmentDestination,
    onCompleted: (importedCount: Int) -> Unit,
  ): Boolean =
    try {
      importAtSelection(session, items, destination, onCompleted)
    } catch (error: Throwable) {
      items.closeAll()
      throw error
    }

  private suspend fun importAtSelection(
    session: DocumentEditingSession,
    items: List<IncomingContentItem>,
    destination: EditorAttachmentDestination,
    onCompleted: (importedCount: Int) -> Unit,
  ): Boolean {
    if (items.isEmpty()) {
      backgroundScope.launch(start = CoroutineStart.UNDISPATCHED) {
        yield()
        onCompleted(0)
      }
      return false
    }
    if (!isSessionCurrent(session)) {
      items.closeAll()
      backgroundScope.launch(start = CoroutineStart.UNDISPATCHED) {
        yield()
        onCompleted(0)
      }
      return false
    }

    val editor = session.editor
    val command = session.submit { _, context ->
      editor.scope.async(context) { mapTargets(session, editor, items, destination) }
    }
    if (command == null) {
      items.closeAll()
      backgroundScope.launch(start = CoroutineStart.UNDISPATCHED) {
        yield()
        onCompleted(0)
      }
      return false
    }

    val targets = command.await()
    items.filterNot { item -> targets.any { target -> target.item === item } }.closeAll()
    if (targets.isEmpty()) {
      backgroundScope.launch(start = CoroutineStart.UNDISPATCHED) {
        yield()
        onCompleted(0)
      }
      return false
    }

    targets.forEach(::setPending)
    backgroundScope.launch(start = CoroutineStart.UNDISPATCHED) {
      val importedCount =
        try {
          yield()
          completeMappedImport(session, editor, targets)
        } finally {
          targets.forEach(::clearPending)
          targets.map(Target::item).closeAll()
        }
      onCompleted(importedCount)
    }
    return true
  }

  private suspend fun completeMappedImport(
    session: DocumentEditingSession,
    editor: Editor,
    targets: List<Target>,
  ): Int = supervisorScope {
    targets
      .map { target ->
        async {
          try {
            uploadTarget(session, editor, target)
          } catch (error: CancellationException) {
            throw error
          } catch (error: Throwable) {
            reportAttachmentFailure(target.item.kind.toAttachmentKind(), error)
            false
          }
        }
      }
      .awaitAll()
      .count { it }
  }

  private suspend fun mapTargets(
    session: DocumentEditingSession,
    editor: Editor,
    items: List<IncomingContentItem>,
    destination: EditorAttachmentDestination,
  ): List<Target> {
    val prepared = items.mapNotNull { item ->
      item.pendingTokenOrNull()?.let { pending -> PreparedItem(item, pending) }
    }
    val targets = mutableListOf<Pair<String, PreparedItem>>()
    var remaining = prepared

    if (destination is EditorAttachmentDestination.ExistingPlaceholder) {
      val first = prepared.firstOrNull() ?: return emptyList()
      if (
        first.item.kind != destination.expectedKind ||
          !isAvailablePlaceholder(editor, destination.nodeId, destination.expectedKind)
      ) {
        return emptyList()
      }
      targets += destination.nodeId to first
      remaining = prepared.drop(1)
    }

    if (remaining.isNotEmpty()) {
      val requestId = Uuid.random().toHexString()
      val requestKinds = remaining.map { it.item.kind.toPlaceholderKind() }
      val nodeIds =
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
            check(matches.size == 1) {
              "Expected exactly one attachment placeholder result for $requestId, got ${matches.size}"
            }
            matches.single().nodeIds
          },
        ) {
          if (editor.ime?.composing != null) {
            enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
          }
          enqueue(Message.Insertion(InsertionOp.AttachmentPlaceholders(requestId, requestKinds)))
        } ?: return emptyList()
      check(remaining.size == nodeIds.size) {
        "Expected ${remaining.size} attachment placeholders, got ${nodeIds.size}"
      }
      targets += nodeIds.zip(remaining)
    }

    return targets.map { (nodeId, preparedItem) ->
      Target(nodeId, preparedItem.item, preparedItem.pending)
    }
  }

  private suspend fun uploadTarget(
    session: DocumentEditingSession,
    editor: Editor,
    target: Target,
  ): Boolean {
    if (!isCurrent(session, editor, target)) return false
    var committed = false
    return try {
      val completed =
        completeAttachmentOperation(
          persist = {
            when (target.item.kind) {
              IncomingContentItem.Kind.Image -> persistence.persistImage(target.item.file)
              IncomingContentItem.Kind.File -> persistence.persistFile(target.item.file)
            }
          },
          isCurrent = { isCurrent(session, editor, target) },
          cache = externalElementState::put,
          commit = { uploaded -> committed = commit(editor, session, target, uploaded) },
          clearPending = { clearPending(target) },
        ) != null
      completed && committed
    } finally {
      clearPending(target)
    }
  }

  private suspend fun commit(
    editor: Editor,
    session: DocumentEditingSession,
    target: Target,
    asset: EditorExternalAsset,
  ): Boolean {
    val operation =
      session.submit { _, context ->
        editor.scope.async(context) {
          editor.await(
            admit = { isCurrent(session, editor, target) },
            beforeCommit = { state ->
              bringIntoViewRequests.requestForVersion(
                target = EditorBringIntoViewTarget.CurrentSelectionHead,
                version = state.version,
              )
            },
          ) {
            enqueue(
              when (target.item.kind) {
                IncomingContentItem.Kind.Image ->
                  Message.Node(
                    NodeOp.SetAttr(
                      id = target.nodeId,
                      attr = NodeAttr.Image(ImageNodeAttr.Id(asset.id)),
                    )
                  )
                IncomingContentItem.Kind.File ->
                  Message.Node(
                    NodeOp.SetAttrs(id = target.nodeId, attrs = PlainNode.File(id = asset.id))
                  )
              }
            )
          }
        }
      } ?: return false
    return operation.await()
  }

  private fun IncomingContentItem.pendingTokenOrNull(): Any? =
    when (kind) {
      IncomingContentItem.Kind.Image -> {
        val width = file.imageWidth ?: return null
        val height = file.imageHeight ?: return null
        if (width <= 0 || height <= 0) return null
        EditorImageUpload(
          previewModel = file.previewModel,
          name = file.filename,
          width = width,
          height = height,
        )
      }
      IncomingContentItem.Kind.File -> EditorFileUpload(name = file.filename, size = file.size)
    }

  private fun setPending(target: Target) {
    when (target.item.kind) {
      IncomingContentItem.Kind.Image ->
        externalElementState.images.uploads[target.nodeId] = target.pending as EditorImageUpload
      IncomingContentItem.Kind.File ->
        externalElementState.files.uploads[target.nodeId] = target.pending as EditorFileUpload
    }
  }

  private fun clearPending(target: Target) {
    when (target.item.kind) {
      IncomingContentItem.Kind.Image -> {
        if (externalElementState.images.uploads[target.nodeId] === target.pending) {
          externalElementState.images.uploads.remove(target.nodeId)
        }
      }
      IncomingContentItem.Kind.File -> {
        if (externalElementState.files.uploads[target.nodeId] === target.pending) {
          externalElementState.files.uploads.remove(target.nodeId)
        }
      }
    }
  }

  private fun isCurrent(session: DocumentEditingSession, editor: Editor, target: Target): Boolean =
    isSessionCurrent(session) &&
      pendingMatches(target) &&
      isEmptyPlaceholder(editor, target.nodeId, target.item.kind)

  private fun pendingMatches(target: Target): Boolean =
    when (target.item.kind) {
      IncomingContentItem.Kind.Image ->
        externalElementState.images.uploads[target.nodeId] === target.pending
      IncomingContentItem.Kind.File ->
        externalElementState.files.uploads[target.nodeId] === target.pending
    }

  private fun isAvailablePlaceholder(
    editor: Editor,
    nodeId: String,
    kind: IncomingContentItem.Kind,
  ): Boolean =
    isEmptyPlaceholder(editor, nodeId, kind) &&
      when (kind) {
        IncomingContentItem.Kind.Image -> !externalElementState.images.uploads.containsKey(nodeId)
        IncomingContentItem.Kind.File -> !externalElementState.files.uploads.containsKey(nodeId)
      }

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
