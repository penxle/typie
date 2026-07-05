package co.typie.screen.editor.editor.aifeedback

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
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.toPageRectsTarget
import co.typie.graphql.AiFeedback_LiteraryAnalysisDocumentStream_Subscription
import co.typie.graphql.Apollo
import co.typie.screen.editor.editor.state.EditorOverlayOcclusion
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import kotlin.math.max
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch

@Stable
internal class EditorAiFeedbackSession(
  val model: AiFeedbackViewModel?,
  val active: Boolean,
  val occlusion: EditorOverlayOcclusion,
  val setOverlayBottomOcclusion: (Float) -> Unit,
  val openFromToolPanel: () -> Unit,
  val close: () -> Unit,
  val rerun: () -> Unit,
  val activateResult: (String) -> Unit,
  val showCurrentResult: (String) -> Unit,
  val ignore: (String) -> Unit,
  val setExpanded: (Boolean) -> Unit,
)

@Composable
internal fun rememberEditorAiFeedbackSession(
  documentId: String?,
  editor: Editor?,
  editorState: EditorState,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  closeIncompatibleModes: () -> Unit,
  ensureSubscription: suspend () -> Boolean,
  ensureAiOptIn: suspend () -> Boolean,
): EditorAiFeedbackSession {
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  var bottomOcclusion by remember(documentId) { mutableFloatStateOf(0f) }
  var lastSelectionMappedToAiFeedback by remember(documentId) { mutableStateOf<Selection?>(null) }
  var occlusionReleaseJob by remember(documentId) { mutableStateOf<Job?>(null) }
  var analysisJob by remember(documentId) { mutableStateOf<Job?>(null) }
  val model = documentId?.let { id ->
    viewModel(key = "editor-ai-feedback:$id") { AiFeedbackViewModel() }
  }
  val active = model?.active == true

  fun setOverlayBottomOcclusion(value: Float) {
    bottomOcclusion = value.coerceAtLeast(0f)
  }

  fun requestRangeIntoView(id: String?) {
    val activeEditor = editor ?: return
    val target = activeEditor.state.trackedRanges.aiFeedbackScrollTarget(id) ?: return
    bringIntoViewRequests.requestForVersion(
      target = target,
      version = activeEditor.state.version,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )
  }

  fun updateActiveRangeDecoration() {
    val activeEditor = editor ?: return
    activeEditor.setActiveAiFeedbackRange(
      activeId = model?.activeRangeId,
      currentRanges = activeEditor.state.trackedRanges,
    )
  }

  fun updateCompactOverlayHeightForRange(id: String?) {
    setOverlayBottomOcclusion(aiFeedbackCompactOverlayHeight(activeRange = id != null).value)
  }

  fun cancelAnalysisState(clearRanges: Boolean) {
    analysisJob?.cancel()
    analysisJob = null
    model?.cancelAnalysis()
    if (clearRanges) {
      editor?.clearAiFeedbackRanges()
    }
    setOverlayBottomOcclusion(0f)
  }

  fun runAnalysis() {
    val activeModel = model ?: return
    val activeEditor = editor ?: return
    val sourceText = activeEditor.proseText()

    analysisJob?.cancel()
    val analysisRunId = activeModel.prepareAnalysis(sourceText)
    activeEditor.installAiFeedbackDecorations()
    activeEditor.clearAiFeedbackRanges()
    lastSelectionMappedToAiFeedback = activeEditor.state.selection
    if (sourceText.trim().isBlank()) {
      activeModel.complete()
      setOverlayBottomOcclusion(0f)
      toast.show(ToastType.Success, "피드백이 없습니다.")
      return
    }

    analysisJob = scope.launch {
      try {
        Apollo.subscription(
            AiFeedback_LiteraryAnalysisDocumentStream_Subscription(text = sourceText)
          )
          .toFlow()
          .collect { response ->
            if (!activeModel.isCurrentAnalysisRun(analysisRunId)) return@collect
            if (activeModel.isPendingAnalysisStale(sourceText, activeEditor.proseText())) {
              cancelAnalysisState(clearRanges = true)
              if (activeModel.active) {
                toast.show(ToastType.Success, "내용이 수정되어 분석이 취소됐어요.")
              }
              return@collect
            }

            val payload = response.data?.literaryAnalysisDocumentStreamV2 ?: return@collect
            when (payload.type) {
              "feedback" -> {
                val raw = payload.feedback?.toRawAiFeedbackResult() ?: return@collect
                if (activeModel.results.any { it.id == raw.id }) return@collect
                val selection = activeEditor.proseToSelection(raw.start, raw.end) ?: return@collect
                val wasEmpty = activeModel.results.isEmpty()
                activeEditor.addAiFeedbackRange(
                  AiFeedbackRangeRegistration(id = raw.id, selection = selection)
                )
                activeModel.appendResult(raw.toAiFeedbackResult())
                if (wasEmpty) {
                  lastSelectionMappedToAiFeedback = activeEditor.state.selection
                }
                updateCompactOverlayHeightForRange(activeModel.activeRangeId)
                updateActiveRangeDecoration()
                if (wasEmpty) {
                  requestRangeIntoView(activeModel.activeRangeId)
                }
              }
              "progress" -> {
                activeModel.updateProgress(payload.progress?.toAiFeedbackProgress())
              }
              "complete" -> {
                activeModel.complete()
                val completedJob = analysisJob
                analysisJob = null
                completedJob?.cancel()
                if (activeModel.results.isEmpty()) {
                  setOverlayBottomOcclusion(0f)
                  toast.show(ToastType.Success, "피드백이 없습니다.")
                }
              }
              "error" -> {
                activeModel.fail()
                val failedJob = analysisJob
                analysisJob = null
                failedJob?.cancel()
                if (activeModel.active) {
                  toast.show(ToastType.Error, "분석에 실패했어요.")
                }
              }
            }
          }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        if (activeModel.isCurrentAnalysisRun(analysisRunId)) {
          activeModel.fail()
          if (activeModel.active) {
            toast.show(ToastType.Error, "분석에 실패했어요.")
          }
        }
      } finally {
        if (analysisJob == coroutineContext[Job]) {
          analysisJob = null
        }
      }
    }
  }

  fun close() {
    val activeEditor = editor
    analysisJob?.cancel()
    analysisJob = null
    model?.exitMode()
    activeEditor?.clearAiFeedbackRanges()
    occlusionReleaseJob?.cancel()
    occlusionReleaseJob = null
    if (bottomOcclusion > 0f) {
      occlusionReleaseJob = scope.launch {
        delay(AiFeedbackOverlayAnimationMillis.toLong())
        bottomOcclusion = 0f
        occlusionReleaseJob = null
      }
    }
  }

  fun disposeEditor(activeEditor: Editor?) {
    if (activeEditor == null) return
    analysisJob?.cancel()
    analysisJob = null
    model?.exitMode()
    activeEditor.clearAiFeedbackRanges()
    occlusionReleaseJob?.cancel()
    occlusionReleaseJob = null
    bottomOcclusion = 0f
    lastSelectionMappedToAiFeedback = null
  }

  DisposableEffect(editor) { onDispose { disposeEditor(editor) } }

  LaunchedEffect(active, editor) {
    if (active) {
      editor?.installAiFeedbackDecorations()
    }
  }

  LaunchedEffect(active, editorState.version) {
    val activeModel = model ?: return@LaunchedEffect
    val expectedText = activeModel.pendingAnalysisText ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    if (!active || !activeModel.loading) return@LaunchedEffect
    if (activeEditor.proseText() == expectedText) return@LaunchedEffect

    cancelAnalysisState(clearRanges = true)
    toast.show(ToastType.Success, "내용이 수정되어 분석이 취소됐어요.")
  }

  LaunchedEffect(active, editorState.trackedRanges, model?.results) {
    val activeModel = model ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    if (!active || activeModel.results.isEmpty()) return@LaunchedEffect

    val removedIds =
      activeModel.cleanupMissingRanges(
        liveIds =
          activeEditor.state.trackedRanges.aiFeedbackRanges().mapTo(mutableSetOf()) { it.id }
      )
    if (removedIds.isEmpty()) return@LaunchedEffect

    if (activeModel.results.isNotEmpty()) {
      updateCompactOverlayHeightForRange(activeModel.activeRangeId)
    }
    updateActiveRangeDecoration()
  }

  LaunchedEffect(active, editorState.selection) {
    val activeModel = model ?: return@LaunchedEffect
    if (!active) {
      lastSelectionMappedToAiFeedback = null
      return@LaunchedEffect
    }
    val selection =
      editorState.selection
        ?: run {
          lastSelectionMappedToAiFeedback = null
          return@LaunchedEffect
        }
    if (activeModel.results.isEmpty()) {
      lastSelectionMappedToAiFeedback = selection
      return@LaunchedEffect
    }
    if (selection == lastSelectionMappedToAiFeedback) return@LaunchedEffect
    lastSelectionMappedToAiFeedback = selection
    if (selection.anchor != selection.head) return@LaunchedEffect

    val rangeId =
      editorState.trackedRangesContainingSelectionHead
        .aiFeedbackRangeEndpoints()
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

  return EditorAiFeedbackSession(
    model = model,
    active = active,
    occlusion =
      if (bottomOcclusion > 0f) {
        EditorOverlayOcclusion(
          bottom = bottomOcclusion,
          bottomScrollReserve =
            max(bottomOcclusion, aiFeedbackCompactOverlayHeight(activeRange = true).value),
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
          if (!ensureAiOptIn()) return@launch
          if (activeModel.active) {
            close()
            return@launch
          }
          occlusionReleaseJob?.cancel()
          occlusionReleaseJob = null
          closeIncompatibleModes()
          activeModel.enterMode()
          runAnalysis()
        }
      },
    close = ::close,
    rerun = rerun@{
        val activeModel = model ?: return@rerun
        if (!activeModel.active) return@rerun
        activeModel.updateExpanded(false)
        runAnalysis()
      },
    activateResult = { id ->
      model?.activate(id)
      updateCompactOverlayHeightForRange(model?.activeRangeId)
      updateActiveRangeDecoration()
      requestRangeIntoView(id)
    },
    showCurrentResult = { id -> model?.setCurrent(id) },
    ignore = ignore@{ id ->
        val activeEditor = editor ?: return@ignore
        activeEditor.removeAiFeedbackRange(id)
        val nextId = model?.remove(id, activateReplacement = true)
        if (nextId != null) {
          updateCompactOverlayHeightForRange(nextId)
        } else {
          setOverlayBottomOcclusion(0f)
        }
        updateActiveRangeDecoration()
        requestRangeIntoView(nextId)
      },
    setExpanded = { expanded -> model?.updateExpanded(expanded) },
  )
}

private fun AiFeedback_LiteraryAnalysisDocumentStream_Subscription.Feedback.toRawAiFeedbackResult():
  RawAiFeedbackResult =
  RawAiFeedbackResult(
    id = id,
    start = start,
    end = end,
    startText = startText,
    endText = endText,
    feedback = feedback,
    category = category,
  )

private fun AiFeedback_LiteraryAnalysisDocumentStream_Subscription.Progress.toAiFeedbackProgress():
  AiFeedbackProgress = AiFeedbackProgress(current = current, total = total, phase = phase)

private fun RawAiFeedbackResult.toAiFeedbackResult(): AiFeedbackResult =
  AiFeedbackResult(
    id = id,
    startText = startText,
    endText = endText,
    feedback = feedback,
    category = category,
  )

private fun List<TrackedRange>.aiFeedbackScrollTarget(id: String?): EditorBringIntoViewTarget? {
  if (id == null) return null
  return aiFeedbackRanges().firstOrNull { it.id == id }?.rects?.toPageRectsTarget()
}
