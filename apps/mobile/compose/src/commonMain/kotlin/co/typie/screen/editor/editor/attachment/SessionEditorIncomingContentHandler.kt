package co.typie.screen.editor.editor.attachment

import co.typie.editor.DocumentEditingSession
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import co.typie.editor.input.EditorIncomingContentHandler
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import co.typie.platform.IncomingContentCandidates
import co.typie.platform.IncomingContentItem
import co.typie.platform.IncomingContentMode
import co.typie.platform.SelectedIncomingContent
import kotlinx.coroutines.async

internal class SessionEditorIncomingContentHandler(
  private val importer: EditorAttachmentImporter,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val isSessionCurrent: (DocumentEditingSession) -> Boolean,
  private val onAttachmentError: (String) -> Unit,
) : EditorIncomingContentHandler {
  override suspend fun handleClipboard(
    session: DocumentEditingSession,
    clipboard: Clipboard,
    mode: IncomingContentMode,
  ): Boolean {
    if (!isSessionCurrent(session)) return false
    val candidates = clipboard.paste() ?: return false
    return handleCandidates(session, candidates, mode)
  }

  override suspend fun handleCandidates(
    session: DocumentEditingSession,
    candidates: IncomingContentCandidates,
    mode: IncomingContentMode,
  ): Boolean {
    if (!isSessionCurrent(session)) {
      candidates.close()
      return false
    }

    return when (val selected = candidates.select(mode)) {
      is SelectedIncomingContent.RichText -> pasteRichText(session, selected)
      is SelectedIncomingContent.Attachments -> {
        val requestedCount = selected.items.size + selected.unreadableItemCount
        val hasImages = selected.items.any { it.kind == IncomingContentItem.Kind.Image }
        val hasFiles = selected.items.any { it.kind == IncomingContentItem.Kind.File }
        val onCompleted: (Int) -> Unit = { importedCount ->
          if (importedCount < requestedCount) {
            onAttachmentError(
              when {
                selected.unreadableItemCount > 0 ->
                  if (importedCount == 0) {
                    "첨부 파일을 삽입하지 못했어요."
                  } else {
                    "일부 첨부 파일을 삽입하지 못했어요."
                  }
                hasImages && !hasFiles ->
                  if (importedCount == 0) {
                    "이미지를 삽입하지 못했어요."
                  } else {
                    "일부 이미지를 삽입하지 못했어요."
                  }
                hasFiles && !hasImages ->
                  if (importedCount == 0) {
                    "파일을 삽입하지 못했어요."
                  } else {
                    "일부 파일을 삽입하지 못했어요."
                  }
                else ->
                  if (importedCount == 0) {
                    "첨부 파일을 삽입하지 못했어요."
                  } else {
                    "일부 첨부 파일을 삽입하지 못했어요."
                  }
              }
            )
          }
        }
        importer.import(
          session = session,
          items = selected.items,
          destination = EditorAttachmentDestination.CurrentSelection,
          onCompleted = onCompleted,
        )
      }
      SelectedIncomingContent.None -> false
    }
  }

  private suspend fun pasteRichText(
    session: DocumentEditingSession,
    content: SelectedIncomingContent.RichText,
  ): Boolean {
    val operation =
      session.submit { editor, context ->
        editor.scope.async(context) {
          editor.await(
            admit = { isSessionCurrent(session) },
            beforeCommit = { state ->
              bringIntoViewRequests.requestForVersion(
                target = EditorBringIntoViewTarget.CurrentSelectionHead,
                version = state.version,
              )
            },
          ) {
            if (editor.ime?.composing != null) {
              enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
            }
            enqueue(Message.Clipboard(ClipboardOp.Paste(html = content.html, text = content.text)))
          }
        }
      } ?: return false
    return operation.await()
  }
}
