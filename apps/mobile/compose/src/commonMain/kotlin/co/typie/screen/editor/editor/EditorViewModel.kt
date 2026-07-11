package co.typie.screen.editor.editor

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.editor.FontLoader
import co.typie.editor.external.EditorExternalAsset
import co.typie.graphql.Apollo
import co.typie.graphql.EditorScreen_AssetsByIds_Query
import co.typie.graphql.EditorScreen_Query
import co.typie.graphql.EditorScreen_UpdateDocument_Mutation
import co.typie.graphql.EditorScreen_ViewEntity_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.type.EntityType
import co.typie.graphql.type.UpdateDocumentInput
import co.typie.graphql.type.ViewEntityInput
import co.typie.graphql.watchQuery
import com.apollographql.cache.normalized.FetchPolicy
import com.apollographql.cache.normalized.fetchPolicy
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject

class EditorViewModel(val entityId: String) : ViewModel() {
  var titleDraft by mutableStateOf("")
    private set

  var subtitleDraft by mutableStateOf("")
    private set

  private var loadingState by mutableStateOf(true)
  private var serverTitle by mutableStateOf("")
  private var serverSubtitle by mutableStateOf("")
  var debugViewportOverlayVisible by mutableStateOf(false)
    private set

  var debugBodyOverlayVisible by mutableStateOf(false)
    private set

  var debugSurfaceOverlayVisible by mutableStateOf(false)
    private set

  private val headerSaveController = EditorHeaderSaveController(scope = viewModelScope)

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      onInitialData = { data -> viewEntity(data.entity.site.id) },
    ) {
      EditorScreen_Query(entityId = entityId)
    }

  init {
    FontLoader.watchFonts(viewModelScope) {
      (query.state as? QueryState.Success)
        ?.data
        ?.entity
        ?.node
        ?.onDocument
        ?.fontLoader_Document
        ?.fontFamilies
        ?.map { it.fontLoader_FontFamily }
    }
  }

  val toolbarFontFamilies by derivedStateOf {
    query.data.entity.node.onDocument?.toolbarFontFamilies.orEmpty().map {
      it.editorSettingsFontFamily_family
    }
  }

  val graph: ByteArray?
    get() =
      ((query.state as? QueryState.Success)?.data ?: return null)
        .entity
        .node
        .onDocument
        ?.state
        ?.graph

  data class DocumentSyncBaseline(
    val seq: String,
    val heads: ByteArray,
    val durableHeads: ByteArray,
  )

  val syncBaseline: DocumentSyncBaseline?
    get() =
      ((query.state as? QueryState.Success)?.data ?: return null)
        .entity
        .node
        .onDocument
        ?.state
        ?.let {
          DocumentSyncBaseline(seq = it.seq, heads = it.heads, durableHeads = it.durableHeads)
        }

  var reloadGeneration by mutableStateOf(0)
    private set

  fun bumpReloadGeneration() {
    reloadGeneration++
  }

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

  fun toggleDebugViewportOverlay() {
    debugViewportOverlayVisible = !debugViewportOverlayVisible
  }

  fun toggleDebugBodyOverlay() {
    debugBodyOverlayVisible = !debugBodyOverlayVisible
  }

  fun toggleDebugSurfaceOverlay() {
    debugSurfaceOverlayVisible = !debugSurfaceOverlayVisible
  }

  suspend fun flushDrafts() {
    headerSaveController.flush(saveTitle = ::saveTitleNow, saveSubtitle = ::saveSubtitleNow)
  }

  suspend fun flush() {
    flushDrafts()
  }

  suspend fun refetchDocument() {
    Apollo.query(EditorScreen_Query(entityId = entityId))
      .fetchPolicy(FetchPolicy.NetworkOnly)
      .execute()
    query.refetch()
  }

  internal suspend fun resolveExternalAssets(ids: List<String>): List<EditorExternalAsset> {
    if (ids.isEmpty()) {
      return emptyList()
    }

    val data =
      Apollo.query(EditorScreen_AssetsByIds_Query(entityId = entityId, ids = ids))
        .fetchPolicy(FetchPolicy.NetworkOnly)
        .execute()
        .dataOrThrow()
    return data.entity.node.onDocument?.assetsByIds.orEmpty().mapNotNull { asset ->
      asset.editorExternalAsset_asset.toEditorExternalAsset()
    }
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
      // TODO(editor-parity): 에디터 에러 UX가 정리되면 header 저장 실패를 화면 안에서
      // 노출해야 한다.
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

  val documentId: String?
    get() = ((query.state as? QueryState.Success)?.data ?: return null).entity.node.onDocument?.id
}

private fun placeholderData() =
  EditorScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { preferences = JsonObject(emptyMap()) }
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
