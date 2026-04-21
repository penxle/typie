package co.typie.screen.editor.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.toEditorDocumentLayoutSpec
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Modifier
import co.typie.editor.ffi.Node
import co.typie.editor.ffi.NodeEntry
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.graphql.Apollo
import co.typie.graphql.EditorScreen_Query
import co.typie.graphql.EditorScreen_UpdateDocument_Mutation
import co.typie.graphql.EditorScreen_ViewEntity_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.UpdateDocumentInput
import co.typie.graphql.type.ViewEntityInput
import co.typie.graphql.watchQuery
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class EditorViewModel(val entityId: String) : ViewModel() {
  var titleDraft by mutableStateOf("")
    private set

  var subtitleDraft by mutableStateOf("")
    private set

  private var loadingState by mutableStateOf(true)
  private var serverTitle by mutableStateOf("")
  private var serverSubtitle by mutableStateOf("")
  private val headerSaveController = EditorHeaderSaveController(scope = viewModelScope)

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      onInitialData = { data -> viewEntity(data.entity.site.id) },
    ) {
      EditorScreen_Query(entityId = entityId)
    }
  val doc =
    Doc(
      nodes =
        mapOf(
          "0" to
            NodeEntry(
              node = Node.Root,
              modifiers =
                listOf(
                  Modifier.FontFamily("Pretendard"),
                  Modifier.FontWeight(400),
                  Modifier.FontSize(1200),
                  Modifier.LineHeight(160),
                  Modifier.LetterSpacing(0),
                  Modifier.TextColor("black"),
                  Modifier.ParagraphIndent(100),
                  Modifier.BlockGap(100),
                ),
              children = listOf("10", "7"),
            ),
          "10" to
            NodeEntry(node = Node.Blockquote(), parent = "0", children = listOf("1", "3", "5")),
          "1" to NodeEntry(node = Node.Paragraph, parent = "10", children = listOf("2")),
          "2" to NodeEntry(node = Node.Text("ABC"), parent = "1"),
          "3" to NodeEntry(node = Node.Paragraph, parent = "10", children = listOf("4")),
          "4" to NodeEntry(node = Node.Text("Hello, World!"), parent = "3"),
          "5" to NodeEntry(node = Node.Paragraph, parent = "10", children = listOf("6")),
          "6" to NodeEntry(node = Node.Text("안녕하세요!"), parent = "5"),
          "7" to NodeEntry(node = Node.Paragraph, parent = "0"),
        ),
      attrs = DocumentAttrs(layoutMode = LayoutMode.Continuous(maxWidth = 600f)),
    )

  val selection = Selection(anchor = Position("4", 0), head = Position("4", 0))

  internal val documentLayoutSpec: EditorDocumentLayoutSpec
    get() = doc.attrs.layoutMode.toEditorDocumentLayoutSpec()

  val headingTitle: String
    get() = if (loadingState && serverTitle.isEmpty() && !isTitleDirty) "" else titleDraft

  val headingSubtitle: String?
    get() =
      when {
        loadingState && serverSubtitle.isEmpty() && !isSubtitleDirty -> null
        subtitleDraft.isBlank() -> null
        else -> subtitleDraft
      }

  fun syncDocument(serverTitle: String?, serverSubtitle: String?, loading: Boolean) {
    loadingState = loading
    if (loading) {
      return
    }

    applyServerSnapshot(
      EditorHeaderSnapshot(title = serverTitle.orEmpty(), subtitle = serverSubtitle.orEmpty())
    )
  }

  fun updateTitleDraft(text: String) {
    if (titleDraft == text) {
      return
    }

    titleDraft = text
    scheduleTitleSave()
  }

  fun updateSubtitleDraft(text: String) {
    if (subtitleDraft == text) {
      return
    }

    subtitleDraft = text
    scheduleSubtitleSave()
  }

  suspend fun flushDrafts() {
    headerSaveController.flush(saveTitle = ::saveTitleNow, saveSubtitle = ::saveSubtitleNow)
  }

  fun flushDraftsAsync() {
    viewModelScope.launch { flushDrafts() }
  }

  private fun viewEntity(siteId: String) {
    viewModelScope.launch {
      try {
        Apollo.executeMutation(
          EditorScreen_ViewEntity_Mutation(
            input = ViewEntityInput(entityId = entityId),
            siteId = siteId,
          )
        )
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Sentry.captureException(e)
      }
    }
  }

  override fun onCleared() {
    headerSaveController.cancelPending()
    super.onCleared()
  }

  private fun scheduleTitleSave() {
    headerSaveController.scheduleTitle(shouldSave = isTitleDirty, action = ::saveTitleNow)
  }

  private fun scheduleSubtitleSave() {
    headerSaveController.scheduleSubtitle(shouldSave = isSubtitleDirty, action = ::saveSubtitleNow)
  }

  private suspend fun saveTitleNow() {
    if (!isTitleDirty) {
      return
    }

    val snapshot = persistTitleDraft(titleDraft) ?: return
    applyServerSnapshot(snapshot)
  }

  private suspend fun saveSubtitleNow() {
    if (!isSubtitleDirty) {
      return
    }

    val snapshot = persistSubtitleDraft(subtitleDraft) ?: return
    applyServerSnapshot(snapshot)
  }

  private suspend fun persistTitleDraft(value: String): EditorHeaderSnapshot? {
    val documentId = documentId ?: return null
    return persistDocument(
      input =
        UpdateDocumentInput.Builder().documentId(documentId).title(value.ifEmpty { null }).build()
    )
  }

  private suspend fun persistSubtitleDraft(value: String): EditorHeaderSnapshot? {
    val documentId = documentId ?: return null
    return persistDocument(
      input =
        UpdateDocumentInput.Builder()
          .documentId(documentId)
          .subtitle(value.ifEmpty { null })
          .build()
    )
  }

  private suspend fun persistDocument(input: UpdateDocumentInput): EditorHeaderSnapshot? {
    return try {
      val document =
        Apollo.executeMutation(EditorScreen_UpdateDocument_Mutation(input = input)).updateDocument
      EditorHeaderSnapshot(
        title = document.nullableTitle.orEmpty(),
        subtitle = document.subtitle.orEmpty(),
      )
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      // TODO(editor-parity): Surface header save failures in-screen once editor error UX is
      // designed.
      Sentry.captureException(e)
      null
    }
  }

  private fun applyServerSnapshot(snapshot: EditorHeaderSnapshot) {
    val hasLocalTitleDraft = isTitleDirty
    val hasLocalSubtitleDraft = isSubtitleDirty

    serverTitle = snapshot.title
    serverSubtitle = snapshot.subtitle

    if (!hasLocalTitleDraft || titleDraft == snapshot.title) {
      titleDraft = snapshot.title
    }

    if (!hasLocalSubtitleDraft || subtitleDraft == snapshot.subtitle) {
      subtitleDraft = snapshot.subtitle
    }
  }

  private val isTitleDirty: Boolean
    get() = titleDraft != serverTitle

  private val isSubtitleDirty: Boolean
    get() = subtitleDraft != serverSubtitle

  private val documentId: String?
    get() = ((query.state as? QueryState.Success)?.data ?: return null).entity.node.onDocument?.id
}

private fun placeholderData() =
  EditorScreen_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = "placeholder-editor-entity"
      type = EntityType.DOCUMENT
      icon = "file-text"
      iconColor = "gray"
      node = buildDocument {
        id = "placeholder-editor-document"
        nullableTitle = text(5..12)
        subtitle = text(8..16)
      }
    }
  }

private data class EditorHeaderSnapshot(val title: String, val subtitle: String)

private class EditorHeaderSaveController(
  private val scope: CoroutineScope,
  private val debounceMillis: Long = 200L,
) {
  private var titleSaveJob: Job? = null
  private var subtitleSaveJob: Job? = null

  fun scheduleTitle(shouldSave: Boolean, action: suspend () -> Unit) {
    titleSaveJob = scheduleSave(currentJob = titleSaveJob, shouldSave = shouldSave, action = action)
  }

  fun scheduleSubtitle(shouldSave: Boolean, action: suspend () -> Unit) {
    subtitleSaveJob =
      scheduleSave(currentJob = subtitleSaveJob, shouldSave = shouldSave, action = action)
  }

  suspend fun flush(saveTitle: suspend () -> Unit, saveSubtitle: suspend () -> Unit) {
    cancelPending()
    saveTitle()
    saveSubtitle()
  }

  fun cancelPending() {
    titleSaveJob?.cancel()
    subtitleSaveJob?.cancel()
    titleSaveJob = null
    subtitleSaveJob = null
  }

  private fun scheduleSave(
    currentJob: Job?,
    shouldSave: Boolean,
    action: suspend () -> Unit,
  ): Job? {
    currentJob?.cancel()
    if (!shouldSave) {
      return null
    }

    return scope.launch {
      delay(debounceMillis)
      action()
    }
  }
}
