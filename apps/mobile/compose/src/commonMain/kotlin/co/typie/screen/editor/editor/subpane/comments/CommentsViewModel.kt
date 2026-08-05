package co.typie.screen.editor.editor.subpane.comments

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.editor.ffi.StableSelection
import co.typie.graphql.Apollo
import co.typie.graphql.CommentsSheet_CreateComment_Mutation
import co.typie.graphql.CommentsSheet_CreateThread_Mutation
import co.typie.graphql.CommentsSheet_DeleteComment_Mutation
import co.typie.graphql.CommentsSheet_DeleteThread_Mutation
import co.typie.graphql.CommentsSheet_Open_Query
import co.typie.graphql.CommentsSheet_ResolveThread_Mutation
import co.typie.graphql.CommentsSheet_Resolved_Query
import co.typie.graphql.CommentsSheet_Stream_Subscription
import co.typie.graphql.CommentsSheet_UnresolveThread_Mutation
import co.typie.graphql.CommentsSheet_UpdateComment_Mutation
import co.typie.graphql.QueryState
import co.typie.graphql.executeMutation
import co.typie.graphql.fragment.CommentsSheetThread_thread
import co.typie.graphql.type.CreateDocumentCommentInput
import co.typie.graphql.type.CreateDocumentCommentThreadInput
import co.typie.graphql.type.DeleteDocumentCommentInput
import co.typie.graphql.type.DeleteDocumentCommentThreadInput
import co.typie.graphql.type.ResolveDocumentCommentThreadInput
import co.typie.graphql.type.UnresolveDocumentCommentThreadInput
import co.typie.graphql.type.UpdateDocumentCommentInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.serialization.json
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.encodeToJsonElement

@OptIn(ExperimentalUuidApi::class)
internal class CommentsViewModel(private val entityId: String, private val documentId: String) :
  ViewModel() {
  val threadState = CommentThreadState()
  val clientId: String = Uuid.random().toHexString()

  private val settledThreadsByFilter =
    mutableStateMapOf<CommentFilter, List<CommentsSheetThread_thread>>()
  private var pendingCreatedThread by mutableStateOf<CommentsSheetThread_thread?>(null)
  val openSelectionsById: Map<String, JsonElement>
    get() = openThreads().associate { thread -> thread.id to thread.selection }

  val openQuery =
    Apollo.watchQuery(scope = viewModelScope, resetOnChange = false) {
      CommentsSheet_Open_Query(entityId = entityId)
    }

  val resolvedQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      resetOnChange = false,
      skip = { threadState.filter != CommentFilter.Resolved },
    ) {
      CommentsSheet_Resolved_Query(entityId = entityId)
    }

  init {
    viewModelScope.launch {
      snapshotFlow { openQuery.state }
        .collect { state ->
          if (state is QueryState.Success) {
            settle(filter = CommentFilter.Open, threads = state.data.threads())
          }
        }
    }

    viewModelScope.launch {
      snapshotFlow { resolvedQuery.state }
        .collect { state ->
          if (state is QueryState.Success) {
            settle(filter = CommentFilter.Resolved, threads = state.data.threads())
          }
        }
    }

    viewModelScope.launch {
      Apollo.subscription(
          CommentsSheet_Stream_Subscription(documentId = documentId, clientId = clientId)
        )
        .toFlow()
        .collect { response ->
          if (response.data?.documentCommentStream != null) {
            refetchThreadLists()
          }
        }
    }
  }

  fun updateFilter(filter: CommentFilter) {
    threadState.updateFilter(filter)
  }

  fun threads(filter: CommentFilter): List<CommentsSheetThread_thread> =
    when (filter) {
      CommentFilter.Open -> openThreads()
      CommentFilter.Resolved -> resolvedThreads()
    }

  fun queryState(filter: CommentFilter): QueryState<*> {
    val source = sourceState(filter)
    val requested = filterRequested(filter)
    return when {
      source is QueryState.Error && filter !in settledThreadsByFilter && requested -> source
      source is QueryState.Success<*> -> QueryState.Success(Unit)
      filter in settledThreadsByFilter -> QueryState.Success(Unit)
      requested -> source
      else -> QueryState.Loading
    }
  }

  suspend fun createThread(
    selection: StableSelection,
    content: String,
  ): Result<CommentsSheetThread_thread, Nothing> = result {
    val created =
      Apollo.executeMutation(
          CommentsSheet_CreateThread_Mutation(
            input =
              CreateDocumentCommentThreadInput(
                clientId = clientId,
                content = content,
                documentId = documentId,
                selection = json.encodeToJsonElement(selection),
              )
          )
        )
        .createDocumentCommentThread
        .commentsSheetThread_thread

    pendingCreatedThread = created
    refetchThreadLists()
    created
  }

  suspend fun createComment(
    threadId: String,
    content: String,
  ): Result<CommentsSheetThread_thread, Nothing> = result {
    Apollo.executeMutation(
        CommentsSheet_CreateComment_Mutation(
          input =
            CreateDocumentCommentInput(clientId = clientId, content = content, threadId = threadId)
        )
      )
      .createDocumentComment
      .commentsSheetThread_thread
      .also { refetchThreadLists() }
  }

  suspend fun updateComment(
    commentId: String,
    content: String,
  ): Result<CommentsSheetThread_thread, Nothing> = result {
    Apollo.executeMutation(
        CommentsSheet_UpdateComment_Mutation(
          input =
            UpdateDocumentCommentInput(
              clientId = clientId,
              commentId = commentId,
              content = content,
            )
        )
      )
      .updateDocumentComment
      .commentsSheetThread_thread
      .also { refetchThreadLists() }
  }

  suspend fun deleteComment(commentId: String): Result<CommentsSheetThread_thread, Nothing> =
    result {
      Apollo.executeMutation(
          CommentsSheet_DeleteComment_Mutation(
            input = DeleteDocumentCommentInput(clientId = clientId, commentId = commentId)
          )
        )
        .deleteDocumentComment
        .commentsSheetThread_thread
        .also { refetchThreadLists() }
    }

  suspend fun deleteThread(threadId: String): Result<CommentsSheetThread_thread, Nothing> = result {
    Apollo.executeMutation(
        CommentsSheet_DeleteThread_Mutation(
          input = DeleteDocumentCommentThreadInput(clientId = clientId, threadId = threadId)
        )
      )
      .deleteDocumentCommentThread
      .commentsSheetThread_thread
      .also {
        clearPendingThread(threadId)
        clearActiveThread(threadId)
        refetchThreadLists()
      }
  }

  suspend fun resolveThread(threadId: String): Result<CommentsSheetThread_thread, Nothing> =
    result {
      Apollo.executeMutation(
          CommentsSheet_ResolveThread_Mutation(
            input = ResolveDocumentCommentThreadInput(clientId = clientId, threadId = threadId)
          )
        )
        .resolveDocumentCommentThread
        .commentsSheetThread_thread
        .also {
          clearPendingThread(threadId)
          clearActiveThread(threadId)
          refetchThreadLists()
        }
    }

  suspend fun unresolveThread(threadId: String): Result<CommentsSheetThread_thread, Nothing> =
    result {
      Apollo.executeMutation(
          CommentsSheet_UnresolveThread_Mutation(
            input = UnresolveDocumentCommentThreadInput(clientId = clientId, threadId = threadId)
          )
        )
        .unresolveDocumentCommentThread
        .commentsSheetThread_thread
        .also {
          updateFilter(CommentFilter.Open)
          refetchThreadLists()
        }
    }

  private fun settle(filter: CommentFilter, threads: List<CommentsSheetThread_thread>) {
    val sortedThreads = threads.sortedByServerOrder()
    settledThreadsByFilter[filter] = sortedThreads
    if (filter == CommentFilter.Open) {
      val pendingThreadId = pendingCreatedThread?.id
      if (pendingThreadId != null && sortedThreads.any { it.id == pendingThreadId }) {
        pendingCreatedThread = null
      }
    }

    val ids = sortedThreads.mapTo(mutableSetOf()) { it.id }
    if (filter == CommentFilter.Open) {
      pendingCreatedThread?.id?.let(ids::add)
    }
    val activeThreadId = threadState.activeThreadId
    if (threadState.filter == filter && activeThreadId != null && activeThreadId !in ids) {
      threadState.activateThread(null)
    }
  }

  private fun openThreads(): List<CommentsSheetThread_thread> {
    val state = openQuery.state
    val serverThreads =
      settledThreadsByFilter[CommentFilter.Open]
        ?: if (state is QueryState.Success) {
          state.data.threads().sortedByServerOrder()
        } else {
          emptyList()
        }
    val pendingThread = pendingCreatedThread
    return if (pendingThread != null && serverThreads.none { it.id == pendingThread.id }) {
      (serverThreads + pendingThread).sortedByServerOrder()
    } else {
      serverThreads
    }
  }

  private fun resolvedThreads(): List<CommentsSheetThread_thread> {
    val state = resolvedQuery.state
    return settledThreadsByFilter[CommentFilter.Resolved]
      ?: if (state is QueryState.Success) {
        state.data.threads().sortedByServerOrder()
      } else {
        emptyList()
      }
  }

  private fun clearActiveThread(threadId: String) {
    if (threadState.activeThreadId == threadId) {
      threadState.activateThread(null)
    }
  }

  private fun clearPendingThread(threadId: String) {
    if (pendingCreatedThread?.id == threadId) {
      pendingCreatedThread = null
    }
  }

  private fun refetchThreadLists() {
    openQuery.refetch()
    if (threadState.filter == CommentFilter.Resolved) {
      resolvedQuery.refetch()
    }
  }

  private fun sourceState(filter: CommentFilter): QueryState<*> =
    when (filter) {
      CommentFilter.Open -> openQuery.state
      CommentFilter.Resolved -> resolvedQuery.state
    }

  private fun filterRequested(filter: CommentFilter): Boolean =
    filter == CommentFilter.Open || threadState.filter == CommentFilter.Resolved
}

internal fun CommentsSheet_Open_Query.Data.threads(): List<CommentsSheetThread_thread> =
  entity.node.onDocument?.commentThreads.orEmpty().map { it.commentsSheetThread_thread }

internal fun CommentsSheet_Resolved_Query.Data.threads(): List<CommentsSheetThread_thread> =
  entity.node.onDocument?.commentThreads.orEmpty().map { it.commentsSheetThread_thread }

private fun List<CommentsSheetThread_thread>.sortedByServerOrder():
  List<CommentsSheetThread_thread> =
  sortedWith(compareBy<CommentsSheetThread_thread> { it.createdAt }.thenBy { it.id })
