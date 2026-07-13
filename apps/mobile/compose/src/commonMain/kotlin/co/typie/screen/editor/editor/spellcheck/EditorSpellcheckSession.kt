package co.typie.screen.editor.editor.spellcheck

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.DocumentEditingSession
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
import co.typie.editor.scroll.toPageRectsTarget
import co.typie.screen.editor.editor.state.EditorOverlayOcclusion
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import kotlin.math.max
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Stable
internal class EditorSpellcheckSession(
  val model: SpellcheckViewModel?,
  val active: Boolean,
  val occlusion: EditorOverlayOcclusion,
  val setOverlayBottomOcclusion: (Float) -> Unit,
  val openFromToolPanel: () -> Unit,
  val close: () -> Unit,
  val rerun: () -> Unit,
  val activateResult: (String) -> Unit,
  val showCurrentResult: (String) -> Unit,
  val applySuggestion: (String, String) -> Unit,
  val directEdit: (String) -> Unit,
  val ignore: (String) -> Unit,
  val ignoreSame: (String) -> Unit,
  val setExpanded: (Boolean) -> Unit,
)

@Composable
internal fun rememberEditorSpellcheckSession(
  documentId: String?,
  documentLocked: Boolean,
  editingSession: DocumentEditingSession?,
  editorState: EditorState,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  hideContextMenu: () -> Unit,
  closeSubPane: () -> Unit,
  ensureSubscription: suspend () -> Boolean,
): EditorSpellcheckSession {
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  var bottomOcclusion by remember(documentId) { mutableFloatStateOf(0f) }
  var lastSelectionMappedToSpellcheck by remember(documentId) { mutableStateOf<Selection?>(null) }
  var programmaticSelectionToSkip by remember(documentId) { mutableStateOf<Selection?>(null) }
  var occlusionReleaseJob by remember(documentId) { mutableStateOf<Job?>(null) }
  val model = documentId?.let { id ->
    viewModel(key = "editor-spellcheck:$id") { SpellcheckViewModel() }
  }
  val active = model?.active == true
  val editor = editingSession?.editor

  fun setOverlayBottomOcclusion(value: Float) {
    bottomOcclusion = value.coerceAtLeast(0f)
  }

  fun requestRangeIntoView(id: String?) {
    val activeEditor = editor ?: return
    val target = activeEditor.state.trackedRanges.spellcheckScrollTarget(id) ?: return
    bringIntoViewRequests.requestForVersion(
      target = target,
      version = activeEditor.state.version,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )
  }

  suspend fun updateActiveRangeDecoration() {
    val activeEditor = editor ?: return
    activeEditor.setActiveSpellcheckRange(
      activeId = model?.activeRangeId,
      currentRanges = activeEditor.state.trackedRanges,
    )
  }

  fun updateCompactOverlayHeightForRange(id: String?) {
    setOverlayBottomOcclusion(spellcheckCompactOverlayHeight(activeRange = id != null).value)
  }

  fun runCheck() {
    val activeModel = model ?: return
    val activeDocumentId = documentId
    val activeEditor = editor ?: return

    scope.launch {
      val sourceText = activeEditor.proseText()
      activeModel.prepareCheck(sourceText)
      activeEditor.installSpellcheckDecorations()
      activeEditor.clearSpellcheckRanges()
      activeModel.runCheck(
        documentId = activeDocumentId,
        text = sourceText,
        onRawResults = { rawResults ->
          scope.launch {
            if (activeModel.isPendingCheckStale(sourceText, activeEditor.proseText())) {
              activeModel.cancelCheck()
              if (activeModel.active) {
                toast.show(ToastType.Success, "내용이 수정되어 맞춤법 검사가 취소됐어요.")
              }
              return@launch
            }
            activeModel.clearPendingCheck()
            val mapped = rawResults.mapNotNull { raw ->
              val selection =
                activeEditor.proseToSelection(raw.start, raw.end) ?: return@mapNotNull null
              raw to selection
            }
            val results = mapped.map { (raw, _) -> raw.toSpellcheckResult() }
            activeModel.replaceResults(results)
            lastSelectionMappedToSpellcheck = activeEditor.state.selection
            if (results.isEmpty()) {
              setOverlayBottomOcclusion(0f)
            } else {
              updateCompactOverlayHeightForRange(activeModel.activeRangeId)
            }
            activeEditor.setSpellcheckRanges(
              mapped.map { (raw, selection) ->
                SpellcheckRangeRegistration(id = raw.id, selection = selection)
              }
            )
            updateActiveRangeDecoration()
            requestRangeIntoView(activeModel.activeRangeId)
            if (results.isEmpty()) {
              toast.show(ToastType.Success, "맞춤법 오류가 없습니다.")
            }
          }
        },
        onError = {
          scope.launch {
            val stale = activeModel.isPendingCheckStale(sourceText, activeEditor.proseText())
            activeModel.clearPendingCheck()
            when {
              stale && activeModel.active ->
                toast.show(ToastType.Success, "내용이 수정되어 맞춤법 검사가 취소됐어요.")
              activeModel.active -> toast.show(ToastType.Error, "맞춤법 검사에 실패했어요.")
            }
          }
        },
      )
    }
  }

  fun close() {
    val activeEditor = editor
    model?.exitMode(resetLoader = true)
    activeEditor?.clearSpellcheckRanges()
    occlusionReleaseJob?.cancel()
    occlusionReleaseJob = null
    if (bottomOcclusion > 0f) {
      occlusionReleaseJob = scope.launch {
        delay(SpellcheckOverlayAnimationMillis.toLong())
        bottomOcclusion = 0f
        occlusionReleaseJob = null
      }
    }
  }

  fun disposeEditor(activeEditor: Editor?) {
    if (activeEditor == null) return
    model?.exitMode(resetLoader = true)
    activeEditor.clearSpellcheckRanges()
    occlusionReleaseJob?.cancel()
    occlusionReleaseJob = null
    bottomOcclusion = 0f
    lastSelectionMappedToSpellcheck = null
    programmaticSelectionToSkip = null
  }

  DisposableEffect(editor) { onDispose { disposeEditor(editor) } }

  LaunchedEffect(active, editor) {
    if (active) {
      editor?.installSpellcheckDecorations()
    }
  }

  LaunchedEffect(active, editorState.version) {
    val activeModel = model ?: return@LaunchedEffect
    val expectedText = activeModel.pendingCheckText ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    if (!active || !activeModel.check.loading) return@LaunchedEffect
    if (activeEditor.proseText() == expectedText) return@LaunchedEffect

    activeModel.cancelCheck()
    toast.show(ToastType.Success, "내용이 수정되어 맞춤법 검사가 취소됐어요.")
  }

  LaunchedEffect(
    active,
    editorState.selection,
    editorState.trackedRanges,
    editorState.trackedRangesContainingSelectionHead,
    model?.results,
  ) {
    val activeModel = model ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    if (!active || activeModel.results.isEmpty()) {
      lastSelectionMappedToSpellcheck = null
      return@LaunchedEffect
    }

    val cleanup =
      activeModel.cleanupStale(
        activeEditor.state.trackedRanges.spellcheckRanges().associate { it.id to it.text }
      )
    if (cleanup.isNotEmpty()) {
      activeEditor.removeSpellcheckRanges(cleanup)
      if (activeModel.results.isNotEmpty()) {
        updateCompactOverlayHeightForRange(activeModel.activeRangeId)
      }
      updateActiveRangeDecoration()
    }

    if (!active || activeModel.results.isEmpty()) {
      lastSelectionMappedToSpellcheck = null
      return@LaunchedEffect
    }
    val selection =
      editorState.selection
        ?: run {
          lastSelectionMappedToSpellcheck = null
          return@LaunchedEffect
        }
    if (selection == programmaticSelectionToSkip) {
      programmaticSelectionToSkip = null
      lastSelectionMappedToSpellcheck = selection
      return@LaunchedEffect
    }
    if (selection == lastSelectionMappedToSpellcheck) return@LaunchedEffect
    lastSelectionMappedToSpellcheck = selection
    if (selection.anchor != selection.head) return@LaunchedEffect

    val rangeId =
      editorState.trackedRangesContainingSelectionHead
        .spellcheckRangeEndpoints()
        .firstOrNull()
        ?.id
        ?.takeIf { id -> activeModel.results.any { it.id == id } }
    val previousActiveRangeId = activeModel.activeRangeId
    if (rangeId == null) {
      activeModel.activate(null)
    } else {
      activeModel.activate(rangeId)
    }
    updateCompactOverlayHeightForRange(activeModel.activeRangeId)
    updateActiveRangeDecoration()
    if (rangeId != null && rangeId != previousActiveRangeId) {
      requestRangeIntoView(rangeId)
    }
  }

  return EditorSpellcheckSession(
    model = model,
    active = active,
    occlusion =
      if (bottomOcclusion > 0f) {
        EditorOverlayOcclusion(
          bottom = bottomOcclusion,
          bottomScrollReserve =
            max(bottomOcclusion, spellcheckCompactOverlayHeight(activeRange = true).value),
        )
      } else {
        EditorOverlayOcclusion()
      },
    setOverlayBottomOcclusion = ::setOverlayBottomOcclusion,
    openFromToolPanel = open@{
        val activeModel = model ?: return@open
        if (activeModel.active) {
          close()
          return@open
        }
        scope.launch {
          if (editor == null) return@launch
          if (!ensureSubscription()) return@launch
          if (activeModel.active) {
            close()
            return@launch
          }
          occlusionReleaseJob?.cancel()
          occlusionReleaseJob = null
          hideContextMenu()
          closeSubPane()
          activeModel.enterMode()
          runCheck()
        }
      },
    close = ::close,
    rerun = rerun@{
        val activeModel = model ?: return@rerun
        if (!activeModel.active) return@rerun
        activeModel.updateExpanded(false)
        runCheck()
      },
    activateResult = { id ->
      model?.activate(id)
      updateCompactOverlayHeightForRange(model?.activeRangeId)
      scope.launch {
        updateActiveRangeDecoration()
        requestRangeIntoView(id)
      }
    },
    showCurrentResult = { id -> model?.setCurrent(id) },
    applySuggestion = applySuggestion@{ id, replacement ->
        val result = model?.results?.firstOrNull { it.id == id } ?: return@applySuggestion
        val activeSession = editingSession ?: return@applySuggestion
        if (documentLocked) {
          toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.")
          return@applySuggestion
        }

        activeSession.submit { activeEditor, context ->
          activeEditor.scope.launch(context) {
            activeEditor.replaceSpellcheckRangeText(
              id = id,
              expectedText = result.context,
              replacement = replacement,
            )
            programmaticSelectionToSkip = activeEditor.state.selection
            val nextId = model.remove(id, activateReplacement = true)
            if (nextId != null) {
              updateCompactOverlayHeightForRange(nextId)
            }
            updateActiveRangeDecoration()
            requestRangeIntoView(nextId)
          }
        }
      },
    directEdit = directEdit@{ id ->
        val activeEditor = editor ?: return@directEdit
        val range =
          activeEditor.state.trackedRanges.spellcheckRanges().firstOrNull { it.id == id }
            ?: return@directEdit
        if (documentLocked) {
          toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.")
          return@directEdit
        }

        model?.activate(null)
        updateCompactOverlayHeightForRange(null)
        model?.updateExpanded(false)
        scope.launch {
          updateActiveRangeDecoration()
          activeEditor.awaitWithBringIntoView(bringIntoViewRequests) {
            enqueue(
              Message.Selection(
                SelectionOp.Set(Selection(anchor = range.anchor, head = range.head))
              )
            )
            beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
          }
          activeEditor.focus()
        }
      },
    ignore = ignore@{ id ->
        val activeEditor = editor ?: return@ignore
        scope.launch {
          activeEditor.removeSpellcheckRange(id)
          val nextId = model?.remove(id, activateReplacement = true)
          if (nextId != null) {
            updateCompactOverlayHeightForRange(nextId)
          }
          updateActiveRangeDecoration()
          requestRangeIntoView(nextId)
        }
      },
    ignoreSame = ignoreSame@{ id ->
        val activeModel = model ?: return@ignoreSame
        val context = activeModel.results.firstOrNull { it.id == id }?.context ?: return@ignoreSame
        val ids =
          activeModel.results.filter { it.context == context }.mapTo(mutableSetOf()) { it.id }
        val activeEditor = editor ?: return@ignoreSame
        scope.launch {
          activeEditor.removeSpellcheckRanges(ids)
          val nextId = activeModel.removeByContext(context, activateReplacement = true)
          if (nextId != null) {
            updateCompactOverlayHeightForRange(nextId)
          }
          updateActiveRangeDecoration()
          requestRangeIntoView(nextId)
        }
      },
    setExpanded = { expanded -> model?.updateExpanded(expanded) },
  )
}

private fun RawSpellcheckResult.toSpellcheckResult(): SpellcheckResult =
  SpellcheckResult(id = id, context = context, corrections = corrections, explanation = explanation)

private fun List<TrackedRange>.spellcheckScrollTarget(id: String?): EditorBringIntoViewTarget? {
  if (id == null) return null
  return spellcheckRanges().firstOrNull { it.id == id }?.rects?.toPageRectsTarget()
}
