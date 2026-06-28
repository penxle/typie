package co.typie.screen.editor.editor.subpane.comments

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.ffi.StableSelection

internal enum class CommentFilter {
  Open,
  Resolved,
}

internal data class VirtualCommentThread(val selection: StableSelection, val content: String = "")

private sealed interface CommentDraft {
  val isDirty: Boolean

  data class Virtual(val selection: StableSelection, val content: String = "") : CommentDraft {
    override val isDirty: Boolean
      get() = content.isNotBlank()
  }

  data class Reply(val threadId: String, val content: String) : CommentDraft {
    override val isDirty: Boolean
      get() = content.isNotBlank()
  }

  data class Edit(val commentId: String, val originalContent: String, val content: String) :
    CommentDraft {
    override val isDirty: Boolean
      get() = content != originalContent
  }
}

@Stable
internal class CommentThreadState {
  var filter by mutableStateOf(CommentFilter.Open)
    private set

  var activeThreadId by mutableStateOf<String?>(null)
    private set

  private var draft by mutableStateOf<CommentDraft?>(null)

  val virtualThread: VirtualCommentThread?
    get() =
      (draft as? CommentDraft.Virtual)?.let { draft ->
        VirtualCommentThread(selection = draft.selection, content = draft.content)
      }

  val editingCommentId: String?
    get() = (draft as? CommentDraft.Edit)?.commentId

  val editingText: String
    get() = (draft as? CommentDraft.Edit)?.content.orEmpty()

  val hasDirtyVirtualThread: Boolean
    get() = (draft as? CommentDraft.Virtual)?.isDirty == true

  val hasDirtyEdit: Boolean
    get() = (draft as? CommentDraft.Edit)?.isDirty == true

  val hasUnsavedInput: Boolean
    get() = draft?.isDirty == true

  fun updateFilter(next: CommentFilter) {
    filter = next
  }

  fun activateThread(threadId: String?) {
    if (activeThreadId != threadId || threadId == null) {
      clearThreadDraft()
    }
    activeThreadId = threadId
    if (threadId != null && draft is CommentDraft.Virtual) {
      draft = null
    }
  }

  fun createVirtualThread(selection: StableSelection) {
    filter = CommentFilter.Open
    activeThreadId = null
    draft = CommentDraft.Virtual(selection = selection)
  }

  fun updateVirtualContent(content: String) {
    val current = draft as? CommentDraft.Virtual ?: return
    draft = current.copy(content = content)
  }

  fun clearVirtualThread() {
    if (draft is CommentDraft.Virtual) {
      draft = null
    }
  }

  fun replyText(threadId: String): String =
    (draft as? CommentDraft.Reply)?.takeIf { it.threadId == threadId }?.content.orEmpty()

  fun updateReplyText(threadId: String, text: String) {
    if (draft != null && draft !is CommentDraft.Reply) {
      return
    }
    draft =
      if (text.isBlank()) {
        null
      } else {
        CommentDraft.Reply(threadId = threadId, content = text)
      }
  }

  fun clearReplyText() {
    if (draft is CommentDraft.Reply) {
      draft = null
    }
  }

  fun startEditing(commentId: String, content: String) {
    draft = CommentDraft.Edit(commentId = commentId, originalContent = content, content = content)
  }

  fun updateEditingText(text: String) {
    val current = draft as? CommentDraft.Edit ?: return
    draft = current.copy(content = text)
  }

  fun clearEditing() {
    if (draft is CommentDraft.Edit) {
      draft = null
    }
  }

  fun discardUnsavedInput() {
    draft = null
  }

  private fun clearThreadDraft() {
    if (draft is CommentDraft.Reply || draft is CommentDraft.Edit) {
      draft = null
    }
  }
}

internal fun StableSelection.sameRangeAs(other: StableSelection): Boolean =
  (anchor == other.anchor && head == other.head) || (anchor == other.head && head == other.anchor)
