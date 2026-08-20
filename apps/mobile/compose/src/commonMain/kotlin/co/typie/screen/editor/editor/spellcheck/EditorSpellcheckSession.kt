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
import co.typie.editor.launchEditorEffect
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.revealTrackedItem
import co.typie.editor.scroll.updateWithBringIntoView
import co.typie.screen.editor.editor.selectTrackedRangeMember
import co.typie.screen.editor.editor.state.EditorOverlayOcclusion
import co.typie.screen.editor.editor.trackedRangeMembershipIds
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import kotlin.math.max
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

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
  onEditingIntent: (Editor) -> Boolean,
  admitMutation: (DocumentEditingSession) -> Boolean,
): EditorSpellcheckSession {
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  var bottomOcclusion by remember(documentId) { mutableFloatStateOf(0f) }
  var lastMembershipIdsMappedToSpellcheck by
    remember(documentId) { mutableStateOf<List<String>?>(null) }
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

  suspend fun requestRangeIntoView(id: String?) {
    val activeEditor = editor ?: return
    if (id == null) return
    activeEditor.revealTrackedItem(bringIntoViewRequests, id)
  }

  suspend fun updateActiveRangeDecoration() {
    val activeEditor = editor ?: return
    activeEditor.setActiveSpellcheckRange(
      activeId = model?.activeRangeId,
      currentRanges = activeEditor.appliedState.trackedRanges,
    )
  }

  fun updateCompactOverlayHeightForRange(id: String?) {
    setOverlayBottomOcclusion(spellcheckCompactOverlayHeight(activeRange = id != null).value)
  }

  fun runCheck() {
    val activeModel = model ?: return
    val activeDocumentId = documentId
    val activeEditor = editor ?: return

    activeModel.runCheck(
      documentId = activeDocumentId,
      sourceText = activeEditor::proseText,
      beforeRequest = { run, _ ->
        activeEditor.installSpellcheckDecorations()
        val cleared = activeEditor.clearSpellcheckRanges(admit = { activeModel.isCurrent(run) })
        if (!cleared) throw CancellationException("Spellcheck run superseded")
      },
      prepareResults = { rawResults, run, sourceText ->
        when (
          activeEditor.installSpellcheckRangesFromProse(
            expectedText = sourceText,
            items = rawResults,
            isCurrent = { activeModel.isCurrent(run) },
          )
        ) {
          SpellcheckRangeInstallResult.Ready ->
            rawResults.map(RawSpellcheckResult::toSpellcheckResult)
          SpellcheckRangeInstallResult.Superseded ->
            throw CancellationException("Spellcheck run superseded")
          SpellcheckRangeInstallResult.StaleCurrent -> {
            if (activeModel.cancelCheck(run)) {
              if (activeModel.active) {
                toast.show(ToastType.Success, "내용이 수정되어 맞춤법 검사가 취소됐어요.")
              }
              activeModel.finishCleanup(run)
            }
            throw CancellationException("Spellcheck source changed")
          }
        }
      },
      onReady = { results ->
        if (results.isEmpty()) {
          setOverlayBottomOcclusion(0f)
        } else {
          updateCompactOverlayHeightForRange(activeModel.activeRangeId)
        }
        activeEditor.launchEffect(coroutineScope = scope) {
          requestRangeIntoView(activeModel.activeRangeId)
        }
        if (results.isEmpty()) {
          toast.show(ToastType.Success, "맞춤법 오류가 없습니다.")
        }
      },
      onError = { _, run ->
        val cleared = activeEditor.clearSpellcheckRanges(admit = { activeModel.ownsCleanup(run) })
        if (cleared && activeModel.ownsCleanup(run) && activeModel.active) {
          toast.show(ToastType.Error, "맞춤법 검사에 실패했어요.")
        }
      },
    )
  }

  fun scheduleRangeClear(activeEditor: Editor?, activeModel: SpellcheckViewModel?) {
    if (activeEditor == null || activeModel == null) return
    activeEditor.launchEffect {
      activeEditor.clearSpellcheckRanges(admit = activeModel::hasNoActiveRun)
    }
  }

  fun close() {
    val activeEditor = editor
    val activeModel = model
    activeModel?.exitMode()
    scheduleRangeClear(activeEditor, activeModel)
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
    val activeModel = model
    activeModel?.exitMode()
    scheduleRangeClear(activeEditor, activeModel)
    occlusionReleaseJob?.cancel()
    occlusionReleaseJob = null
    bottomOcclusion = 0f
    lastMembershipIdsMappedToSpellcheck = null
    programmaticSelectionToSkip = null
  }

  DisposableEffect(editor) { onDispose { disposeEditor(editor) } }

  LaunchedEffect(active, editor) {
    if (active) {
      editor?.runEffect { editor.installSpellcheckDecorations() }
    }
  }

  LaunchedEffect(active, editorState.documentRevision) {
    val activeModel = model ?: return@LaunchedEffect
    val pendingCheck = activeModel.pendingCheck ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    if (!active || !activeModel.loading) return@LaunchedEffect
    activeEditor.runEffect effect@{
      if (activeEditor.proseText() == pendingCheck.sourceText) return@effect

      val run = pendingCheck.run
      if (!activeModel.cancelCheck(run)) return@effect
      try {
        val cleared =
          withContext(NonCancellable) {
            activeEditor.clearSpellcheckRanges(admit = { activeModel.ownsCleanup(run) })
          }
        if (cleared && activeModel.ownsCleanup(run) && activeModel.active) {
          toast.show(ToastType.Success, "내용이 수정되어 맞춤법 검사가 취소됐어요.")
        }
      } finally {
        activeModel.finishCleanup(run)
      }
    }
  }

  LaunchedEffect(
    active,
    editorState.selection,
    editorState.trackedRanges,
    editorState.trackedRangesContainingSelection,
    model?.results,
  ) {
    val activeModel = model ?: return@LaunchedEffect
    val activeEditor = editor ?: return@LaunchedEffect
    if (!active || activeModel.results.isEmpty()) {
      lastMembershipIdsMappedToSpellcheck = null
      return@LaunchedEffect
    }

    activeEditor.runEffect effect@{
      val cleanup =
        activeModel.cleanupStale(
          activeEditor.appliedState.trackedRanges.spellcheckRanges().associate { it.id to it.text }
        )
      if (cleanup.isNotEmpty()) {
        activeEditor.removeSpellcheckRanges(cleanup)
        if (activeModel.results.isNotEmpty()) {
          updateCompactOverlayHeightForRange(activeModel.activeRangeId)
        }
        updateActiveRangeDecoration()
      }

      if (!active || activeModel.results.isEmpty()) {
        lastMembershipIdsMappedToSpellcheck = null
        return@effect
      }
      val selection =
        editorState.selection
          ?: run {
            lastMembershipIdsMappedToSpellcheck = null
            return@effect
          }
      val resultIds = activeModel.results.mapTo(mutableSetOf()) { it.id }
      val membershipIds =
        editorState.trackedRangesContainingSelection.trackedRangeMembershipIds(
          allowedGroups = SPELLCHECK_MEMBERSHIP_GROUPS,
          ownedIds = resultIds,
        )
      if (selection == programmaticSelectionToSkip) {
        programmaticSelectionToSkip = null
        lastMembershipIdsMappedToSpellcheck = membershipIds
        return@effect
      }
      if (membershipIds == lastMembershipIdsMappedToSpellcheck) return@effect
      lastMembershipIdsMappedToSpellcheck = membershipIds

      val rangeId =
        editorState.trackedRangesContainingSelection
          .selectTrackedRangeMember(
            allowedGroups = SPELLCHECK_MEMBERSHIP_GROUPS,
            activeId = activeModel.activeRangeId,
            ownedIds = resultIds,
          )
          ?.id
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
        scope.launchEditorEffect(editor) {
          if (editor == null) return@launchEditorEffect
          if (!ensureSubscription()) return@launchEditorEffect
          if (activeModel.active) {
            close()
            return@launchEditorEffect
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
        if (!activeModel.active || activeModel.loading) return@rerun
        scope.launchEditorEffect(editor) {
          if (!ensureSubscription()) return@launchEditorEffect
          if (!activeModel.active || activeModel.loading) return@launchEditorEffect
          activeModel.updateExpanded(false)
          runCheck()
        }
      },
    activateResult = { id ->
      model?.activate(id)
      updateCompactOverlayHeightForRange(model?.activeRangeId)
      scope.launchEditorEffect(editor) {
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

        scope.launchEditorEffect(activeSession.editor) {
          if (activeSession.editor.trackedRange(id) == null) return@launchEditorEffect
          if (!ensureSubscription()) return@launchEditorEffect
          if (!onEditingIntent(activeSession.editor)) return@launchEditorEffect
          activeSession.submit { activeEditor, context ->
            activeEditor.launchEffect(context = context) {
              val replaced =
                activeEditor.replaceSpellcheckRangeText(
                  id = id,
                  expectedText = result.context,
                  replacement = replacement,
                  admit = { admitMutation(activeSession) },
                )
              if (replaced) {
                programmaticSelectionToSkip = activeEditor.appliedState.selection
                val nextId = model.remove(id, activateReplacement = true)
                if (nextId != null) {
                  updateCompactOverlayHeightForRange(nextId)
                }
                updateActiveRangeDecoration()
                requestRangeIntoView(nextId)
              }
            }
          }
        }
      },
    directEdit = directEdit@{ id ->
        val activeSession = editingSession ?: return@directEdit
        val activeEditor = activeSession.editor
        scope.launchEditorEffect(activeEditor) {
          val range = activeEditor.trackedRange(id) ?: return@launchEditorEffect
          if (documentLocked) {
            toast.show(ToastType.Error, "잠긴 문서는 편집할 수 없어요.")
            return@launchEditorEffect
          }

          if (!ensureSubscription()) return@launchEditorEffect
          if (!onEditingIntent(activeEditor)) return@launchEditorEffect
          val update =
            activeEditor.updateWithBringIntoView(
              bringIntoViewRequests = bringIntoViewRequests,
              admit = { admitMutation(activeSession) },
            ) {
              enqueue(
                Message.Selection(
                  SelectionOp.Set(Selection(anchor = range.anchor, head = range.head))
                )
              )
              bringIntoView(
                EditorBringIntoViewTarget.CurrentSelectionHead,
                policy = EditorBringIntoViewPolicy.CursorGuard,
              )
            }
          if (update == null) return@launchEditorEffect
          model?.activate(null)
          updateCompactOverlayHeightForRange(null)
          model?.updateExpanded(false)
          updateActiveRangeDecoration()
          if (admitMutation(activeSession)) {
            activeEditor.focus()
          }
        }
      },
    ignore = ignore@{ id ->
        val activeEditor = editor ?: return@ignore
        scope.launchEditorEffect(activeEditor) {
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
        scope.launchEditorEffect(activeEditor) {
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
