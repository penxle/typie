package co.typie.screen.editor.editor

import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.PlanUpgradeBenefit
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.editor.Editor
import co.typie.editor.EditorLocalChangesetBus
import co.typie.editor.EditorState
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.body.EditorBody
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveBaseBottomSpace
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolvePagesContentHeight
import co.typie.editor.body.toEditorDocumentLayoutSpec
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.HistoryTag
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.allowsViewportScrollReconcile
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.rememberEditorZoomController
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.scroll.resolveBringIntoViewTargetHeight
import co.typie.editor.scroll.resolveDistanceToPagesBottom
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.sync.ActiveSyncEngines
import co.typie.editor.sync.ChangesetDeltaStore
import co.typie.editor.sync.GraphQlSyncTransport
import co.typie.editor.sync.RemoteChangesetPipeline
import co.typie.editor.sync.SyncEngine
import co.typie.editor.sync.SyncSession
import co.typie.editor.sync.asSyncEditor
import co.typie.editor.sync.catchingNonCancellation
import co.typie.editor.sync.isPermanentSyncError
import co.typie.editor.sync.orphanSweeper
import co.typie.editor.sync.syncAppScope
import co.typie.editor.viewport.consumeEditorViewportTouchPan
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ime
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.platform.PlatformModule
import co.typie.platform.connectivityRestoredFlow
import co.typie.route.Route
import co.typie.screen.document.document.DocumentCharacterCountSnapshots
import co.typie.screen.editor.editor.aifeedback.AiFeedbackOptInSheet
import co.typie.screen.editor.editor.aifeedback.AiFeedbackOverlay
import co.typie.screen.editor.editor.aifeedback.AiFeedbackTopBarCenter
import co.typie.screen.editor.editor.aifeedback.AiFeedbackTopBarLeading
import co.typie.screen.editor.editor.aifeedback.AiFeedbackTopBarTrailing
import co.typie.screen.editor.editor.aifeedback.rememberEditorAiFeedbackSession
import co.typie.screen.editor.editor.entry.rememberEditorEntryStateSession
import co.typie.screen.editor.editor.findreplace.FindReplaceToolbar
import co.typie.screen.editor.editor.findreplace.FindReplaceTopBarCenter
import co.typie.screen.editor.editor.findreplace.FindReplaceTopBarLeading
import co.typie.screen.editor.editor.findreplace.FindReplaceTopBarTrailing
import co.typie.screen.editor.editor.findreplace.rememberEditorFindReplaceSession
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.screen.editor.editor.layout.EditorScreenLayout
import co.typie.screen.editor.editor.layout.EditorViewportScrollReconcileMode
import co.typie.screen.editor.editor.overlay.EditorCharacterCountOverlay
import co.typie.screen.editor.editor.overlay.EditorRepasteAsTextOverlay
import co.typie.screen.editor.editor.overlay.EditorScreenOverlayHost
import co.typie.screen.editor.editor.overlay.EditorZoomOverlay
import co.typie.screen.editor.editor.placeholder.EditorDocumentPlaceholder
import co.typie.screen.editor.editor.spellcheck.SpellcheckOverlay
import co.typie.screen.editor.editor.spellcheck.SpellcheckTopBarCenter
import co.typie.screen.editor.editor.spellcheck.SpellcheckTopBarLeading
import co.typie.screen.editor.editor.spellcheck.SpellcheckTopBarTrailing
import co.typie.screen.editor.editor.spellcheck.rememberEditorSpellcheckSession
import co.typie.screen.editor.editor.state.EditorInputEffect
import co.typie.screen.editor.editor.state.EditorOverlayOcclusion
import co.typie.screen.editor.editor.state.rememberEditorScreenState
import co.typie.screen.editor.editor.state.resolveEditorVisibleAreas
import co.typie.screen.editor.editor.subpane.CommentsSubPaneEnvironment
import co.typie.screen.editor.editor.subpane.EditorSubPane
import co.typie.screen.editor.editor.subpane.EditorSubPaneHost
import co.typie.screen.editor.editor.subpane.EditorSubPaneState
import co.typie.screen.editor.editor.subpane.comments.rememberEditorCommentsSession
import co.typie.screen.editor.editor.subpane.resolveSubPaneBottomOcclusion
import co.typie.screen.editor.editor.template.EditorTemplateSheet
import co.typie.screen.editor.editor.toolbar.EditorToolbarDebugOverlays
import co.typie.screen.editor.editor.toolbar.EditorToolbarHost
import co.typie.screen.editor.editor.toolbar.EditorToolbarToolAction
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPadding
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelGap
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityEnterMillis
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityExitMillis
import co.typie.screen.editor.editor.toolbar.ToolbarHeight
import co.typie.screen.editor.editor.toolbar.ToolbarInputEnvironment
import co.typie.screen.editor.editor.toolbar.ToolbarIntent
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryStackHeight
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryVisibilityMillis
import co.typie.screen.editor.editor.toolbar.effectiveImeInset
import co.typie.screen.editor.editor.toolbar.isEditorToolbarPresented
import co.typie.screen.editor.editor.toolbar.isImeVisible
import co.typie.screen.editor.editor.toolbar.rememberEditorKeyboardState
import co.typie.screen.editor.editor.toolbar.rememberEditorToolbarInputState
import co.typie.screen.editor.editor.toolbar.rememberEditorToolbarSessionState
import co.typie.screen.editor.editor.toolbar.rememberToolbarPagerState
import co.typie.screen.editor.editor.toolbar.suppressSoftwareKeyboard
import co.typie.screen.editor.editor.toolbar.textInputSessionEnabledForBottomPanel
import co.typie.screen.editor.editor.toolbar.trustedImeBottomInset
import co.typie.screen.editor.editor.topbar.EditorDocumentButton
import co.typie.screen.editor.editor.viewport.rememberEditorDebugWheelZoomModifier
import co.typie.screen.settings.aisettings.AiPreferences
import co.typie.serialization.json
import co.typie.storage.Preference
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.popover.LocalPopoverOverlayState
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.HazeState
import kotlin.time.Clock
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
import kotlinx.serialization.json.decodeFromJsonElement

@OptIn(ExperimentalComposeUiApi::class, ExperimentalUuidApi::class)
@Composable
fun EditorScreen(entityId: String) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val sheet = LocalSheet.current
  val toast = LocalToast.current
  val model = viewModel { EditorViewModel(entityId) }
  val scope = rememberCoroutineScope()
  val runtime = remember(entityId) { EditorRuntime(uiScope = scope) }
  val interactionScope = remember(entityId) { EditorInteractionScope(coroutineScope = scope) }
  val uiState = remember(entityId) { EditorUiState() }
  val externalElementState = remember(entityId) { EditorExternalElementState() }
  val assetHydrator =
    remember(entityId) {
      EditorAssetHydrator(state = externalElementState, fetch = model::resolveExternalAssets)
    }
  var assetQueryGeneration by remember(entityId) { mutableStateOf(0L) }
  val zoomController = rememberEditorZoomController(key = entityId)
  val screenState = rememberEditorScreenState(key = entityId)
  val subPaneState = remember(entityId) { EditorSubPaneState() }
  val loading = model.query.state !is QueryState.Success
  val entity = model.query.data.entity
  val document = entity.node.onDocument
  val documentLocked = document?.locked == true

  DisposableEffect(model) {
    onDispose {
      interactionScope.reset()
      // TODO(editor-parity): 에디터 스크린 생명주기가 composition dispose 밖까지 연결되면,
      // app background/inactive 전환에서도 header draft를 flush해야 한다.
      model.flushDraftsAsync()
    }
  }
  LaunchedEffect(nav.current, entityId, runtime, screenState) {
    screenState.updateSceneForeground(
      isForeground = nav.current == Route.Editor(entityId),
      runtime = runtime,
      uiState = uiState,
    )
  }
  LaunchedEffect(document?.nullableTitle, document?.subtitle, loading) {
    model.syncDocument(
      serverTitle = document?.nullableTitle,
      serverSubtitle = document?.subtitle,
      loading = loading,
    )
  }
  LaunchedEffect(document?.assets, loading, model.reloadGeneration, assetHydrator) {
    if (!loading) {
      val assets =
        document?.assets.orEmpty().mapNotNull { asset ->
          asset.editorExternalAsset_asset.toEditorExternalAsset()
        }
      assetQueryGeneration += 1
      assetHydrator.onQueryRefresh(generation = assetQueryGeneration, assets = assets)
    }
  }
  LaunchedEffect(runtime.error) {
    runtime.error ?: return@LaunchedEffect
    dialog.error(nav) { runtime.clearError() }
  }
  val legacyDocument = !loading && document != null && document.state == null
  LaunchedEffect(legacyDocument, nav.isTransitioning) {
    if (!legacyDocument || nav.isTransitioning) return@LaunchedEffect
    dialog.error(nav = nav, title = "문서를 열 수 없어요", message = "구버전 문서는 지원하지 않아요.") {
      model.query.refetch()
    }
  }
  fun requestEditorFocus() {
    if (nav.current == Route.Editor(entityId)) {
      runtime.focus()
    }
  }
  val editor = runtime.editor
  val editorState = editor?.state ?: EditorState.Initial
  val externalAssetIds =
    remember(editorState.externalElements) {
      editorState.externalElements
        .mapNotNull { element ->
          when (val data = element.data) {
            is ExternalElementData.Image -> data.id
            is ExternalElementData.File -> data.id
            is ExternalElementData.Embed -> data.id
            is ExternalElementData.Archived -> null
          }
        }
        .distinct()
        .sorted()
    }
  LaunchedEffect(assetHydrator, assetQueryGeneration, externalAssetIds) {
    assetHydrator.resolve(externalAssetIds)
  }
  LaunchedEffect(assetHydrator) {
    var connectivityGeneration = 0L
    connectivityRestoredFlow().collect {
      connectivityGeneration += 1
      assetHydrator.onConnectivityRestored(connectivityGeneration)
    }
  }
  LaunchedEffect(subPaneState.active, editorState.selection) {
    subPaneState.dismissTableAxisActionsIfSelectionChanged(editorState.selection)
  }
  val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
  val entryState =
    rememberEditorEntryStateSession(
      documentId = document?.id,
      editor = editor,
      editorFocused = uiState.focused,
      bringIntoViewRequests = bringIntoViewRequests,
    )
  val findReplace =
    rememberEditorFindReplaceSession(
      documentLocked = documentLocked,
      editor = editor,
      editorState = editorState,
      bringIntoViewRequests = bringIntoViewRequests,
    )
  fun requestEditorFocusIfSelectionActive() {
    if (editorState.selection != null) {
      requestEditorFocus()
    }
  }
  fun closeFindReplaceFromShortcut() {
    findReplace.close()
    requestEditorFocusIfSelectionActive()
  }
  suspend fun ensureSpellcheckSubscription(): Boolean {
    return SubscriptionService.gate(
      sheet = sheet,
      nav = nav,
      title = "맞춤법 검사로\n글을 더 자연스럽게 다듬어요.",
      benefits = listOf(PlanUpgradeBenefit.SpellCheck),
    )
  }
  suspend fun ensureAiFeedbackSubscription(): Boolean {
    return SubscriptionService.gate(
      sheet = sheet,
      nav = nav,
      title = "AI 피드백은 FULL ACCESS 플랜에서 사용할 수 있어요.",
      benefits = listOf(PlanUpgradeBenefit.AiFeedback),
    )
  }
  val aiOptIn =
    remember(model.query.data.me.preferences) {
      runCatching {
          json.decodeFromJsonElement<AiPreferences>(model.query.data.me.preferences).aiOptIn
        }
        .getOrDefault(false)
    }
  suspend fun ensureAiOptIn(): Boolean {
    if (aiOptIn) return true
    val result = sheet.present<Boolean> { AiFeedbackOptInSheet() }
    if (result == true) {
      nav.navigate(Route.AiSettings)
    }
    return false
  }
  val spellcheck =
    rememberEditorSpellcheckSession(
      documentId = document?.id,
      documentLocked = documentLocked,
      editor = editor,
      editorState = editorState,
      bringIntoViewRequests = bringIntoViewRequests,
      hideContextMenu = { uiState.contextMenu.hide() },
      closeSubPane = subPaneState::dismiss,
      ensureSubscription = ::ensureSpellcheckSubscription,
    )
  val aiFeedback =
    rememberEditorAiFeedbackSession(
      documentId = document?.id,
      editor = editor,
      editorState = editorState,
      bringIntoViewRequests = bringIntoViewRequests,
      closeIncompatibleModes = {
        findReplace.close()
        spellcheck.close()
        uiState.contextMenu.hide()
        subPaneState.dismiss()
      },
      ensureSubscription = ::ensureAiFeedbackSubscription,
      ensureAiOptIn = ::ensureAiOptIn,
    )

  when {
    findReplace.active -> {
      ProvideTopBar(
        leading = { FindReplaceTopBarLeading(session = findReplace) },
        leadingKey = FindReplaceTopBarLeadingKey,
        center = {
          FindReplaceTopBarCenter(session = findReplace, onEscape = ::closeFindReplaceFromShortcut)
        },
        centerKey = FindReplaceTopBarCenterKey,
        trailing = { FindReplaceTopBarTrailing(session = findReplace) },
        trailingKey = FindReplaceTopBarTrailingKey,
        scrollOffset = null,
      )
    }
    spellcheck.active -> {
      ProvideTopBar(
        leading = { SpellcheckTopBarLeading(session = spellcheck) },
        leadingKey = SpellcheckTopBarLeadingKey,
        center = { SpellcheckTopBarCenter(session = spellcheck) },
        centerKey = SpellcheckTopBarCenterKey,
        trailing = { SpellcheckTopBarTrailing(session = spellcheck) },
        trailingKey = SpellcheckTopBarTrailingKey,
        scrollOffset = null,
      )
    }
    aiFeedback.active -> {
      ProvideTopBar(
        leading = { AiFeedbackTopBarLeading(session = aiFeedback) },
        leadingKey = AiFeedbackTopBarLeadingKey,
        center = { AiFeedbackTopBarCenter(session = aiFeedback) },
        centerKey = AiFeedbackTopBarCenterKey,
        trailing = { AiFeedbackTopBarTrailing(session = aiFeedback) },
        trailingKey = AiFeedbackTopBarTrailingKey,
        scrollOffset = null,
      )
    }
    else -> {
      ProvideTopBar(
        center = {
          document?.let {
            Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
              EditorDocumentButton(
                entityIcon = entity.entityIcon_entity,
                title = model.headingTitle,
                subtitle = model.headingSubtitle,
                loading = loading,
                onClick = {
                  val activeEditor = runtime.editor
                  val target = Route.Document(entityId)
                  var delivered = false
                  try {
                    DocumentCharacterCountSnapshots.put(entityId, activeEditor?.characterCounts())
                    screenState.prepareToLeaveEditorScene(
                      runtime = runtime,
                      uiState = uiState,
                      flushDrafts = { model.flush() },
                    )
                    nav.navigate(target)
                    delivered = nav.current == target
                  } finally {
                    if (!delivered) {
                      DocumentCharacterCountSnapshots.remove(entityId)
                    }
                  }
                },
                modifier =
                  Modifier.fillMaxWidth().widthIn(max = ResponsiveContainerDefaults.MaxWidth),
              )
            }
          }
        },
        scrollOffset = null,
      )
    }
  }

  val subtitleFocusRequestVersion = remember(entityId) { mutableStateOf(0) }
  DisposableEffect(editor) {
    val off =
      editor?.on<EditorEvent.CursorExitedDocumentStart> { _, _ ->
        subtitleFocusRequestVersion.value += 1
      }
    onDispose { off?.invoke() }
  }
  LaunchedEffect(entityId, editor) {
    val activeEditor = editor ?: return@LaunchedEffect
    EditorLocalChangesetBus.consume(entityId).forEach { activeEditor.receiveRemoteChangeset(it) }
  }
  LaunchedEffect(entityId, runtime) {
    EditorLocalChangesetBus.notifications(entityId).collectLatest {
      val activeEditor = runtime.editor ?: return@collectLatest
      EditorLocalChangesetBus.consume(entityId).forEach { activeEditor.receiveRemoteChangeset(it) }
    }
  }

  val activeEditor = runtime.editor
  val syncBaseline = model.syncBaseline
  val documentId = model.documentId
  if (activeEditor != null && syncBaseline != null && documentId != null) {
    DisposableEffect(activeEditor) {
      val clientId = Uuid.random().toString()
      val transport = GraphQlSyncTransport(documentId = documentId, clientId = clientId)
      val engineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
      val engine =
        SyncEngine(
          editor = activeEditor.asSyncEditor(),
          documentId = documentId,
          initialServerHeads = syncBaseline.heads,
          initialDurableHeads = syncBaseline.durableHeads,
          store = ChangesetDeltaStore,
          pushFn = { transport.push(it) },
          scope = engineScope,
          isPermanent = ::isPermanentSyncError,
          now = { Clock.System.now().toEpochMilliseconds() },
        )
      val pipeline =
        RemoteChangesetPipeline(
          editor = activeEditor.asSyncEditor(),
          headsSink = engine,
          transport = transport,
          initialSeq = syncBaseline.seq,
          scope = scope,
          onNeedsReload = {
            catchingNonCancellation { engine.captureNow() }
            engine.stop()
            model.refetchDocument()
            model.bumpReloadGeneration()
            runtime.clear(activeEditor)
          },
        )
      val session = SyncSession(engine = engine, pipeline = pipeline)
      ActiveSyncEngines.register(documentId, session)
      pipeline.start()

      val revisionJob = scope.launch {
        snapshotFlow { activeEditor.state.documentRevision }.drop(1).collect { engine.schedule() }
      }

      onDispose {
        revisionJob.cancel()
        pipeline.stop()
        engine.stop()
        ActiveSyncEngines.unregister(documentId, session)
        syncAppScope.launch { orphanSweeper.sweep() }
      }
    }
  }

  val layoutSpec: EditorDocumentLayoutSpec =
    editor?.state?.rootAttrs?.layoutMode?.toEditorDocumentLayoutSpec()
      ?: EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
  val background =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Paginated -> AppTheme.colors.surfaceCanvas
      is EditorDocumentLayoutSpec.Continuous -> AppTheme.colors.surfaceDefault
    }

  Screen(loadable = model.query, background = background, contentPadding = PaddingValues()) {
    contentPadding ->
    val comments =
      rememberEditorCommentsSession(
        entityId = entityId,
        documentId = document?.id,
        documentLocked = documentLocked,
        editor = editor,
        editorState = editorState,
        sheetActive = subPaneState.active == EditorSubPane.Comments,
        bringIntoViewRequests = bringIntoViewRequests,
        hideContextMenu = { uiState.contextMenu.hide() },
        openSheet = { subPaneState.open(EditorSubPane.Comments) },
      )
    val pageSizes = editorState.pageSizes
    val density = LocalDensity.current.density
    val focusManager = LocalFocusManager.current
    val haptic = LocalHapticFeedback.current
    val keyboardController = LocalSoftwareKeyboardController.current
    val scrollGestureLockState = LocalScrollGestureLockState.current
    val layoutDirection = LocalLayoutDirection.current
    val topInset = contentPadding.calculateTopPadding()
    val startInset = contentPadding.calculateStartPadding(layoutDirection)
    val endInset = contentPadding.calculateEndPadding(layoutDirection)
    val bottomSafeInset = contentPadding.calculateBottomPadding()
    val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
    val toolbarInputState = rememberEditorToolbarInputState()
    val toolbarBackdropHazeState = remember { HazeState() }
    val keyboardState = rememberEditorKeyboardState()
    val toolbarPagerState = rememberToolbarPagerState(key = entityId)
    val toolbarSessionState = rememberEditorToolbarSessionState(key = entityId)
    val toolbarPanel = toolbarInputState.panel
    val bottomPanelOpen = toolbarPanel != null
    val bottomPanelTransition = remember { MutableTransitionState(bottomPanelOpen) }
    bottomPanelTransition.targetState = bottomPanelOpen
    val panelTransitionRunning =
      bottomPanelTransition.currentState != bottomPanelTransition.targetState
    val subPaneBlocksEditorInput = subPaneState.editorInputBlocked
    val screenShortcutModeActive = findReplace.active || spellcheck.active || aiFeedback.active
    val editorToolbarVisible =
      screenState.sceneInForeground && !subPaneBlocksEditorInput && !findReplace.active
    val findReplaceToolbarVisible =
      screenState.sceneInForeground && !subPaneBlocksEditorInput && findReplace.active
    val findReplaceToolbarTransition = remember {
      MutableTransitionState(findReplaceToolbarVisible)
    }
    findReplaceToolbarTransition.targetState = findReplaceToolbarVisible
    val findReplaceToolbarInLayout =
      findReplaceToolbarVisible || !findReplaceToolbarTransition.isIdle
    val toolbarInputEnvironment =
      ToolbarInputEnvironment(
        visible = editorToolbarVisible,
        focused = uiState.focused,
        imeBottom = imeBottom,
        safeBottomInset = bottomSafeInset,
        keyboardState = keyboardState,
        panelTransitionRunning = panelTransitionRunning,
      )
    fun performInputEffects(effects: List<EditorInputEffect>) {
      effects.forEach { effect ->
        when (effect) {
          EditorInputEffect.ShowKeyboard -> keyboardController?.show()
          EditorInputEffect.HideKeyboard -> keyboardController?.hide()
          EditorInputEffect.RequestFocus -> requestEditorFocus()
          EditorInputEffect.ClearFocus -> focusManager.clearFocus(force = true)
        }
      }
    }
    fun openFindReplace() {
      aiFeedback.close()
      spellcheck.close()
      uiState.contextMenu.hide()
      performInputEffects(toolbarInputState.dispatch(ToolbarIntent.Reset, toolbarInputEnvironment))
      findReplace.open()
    }
    val screenShortcutContext =
      EditorScreenShortcutContext(
        platform = PlatformModule.platform,
        sceneInForeground = screenState.sceneInForeground,
        subPaneBlocksEditorInput = subPaneBlocksEditorInput,
        editorFocused = uiState.focused,
        findReplaceActive = findReplace.active,
        spellcheckActive = spellcheck.active,
        aiFeedbackActive = aiFeedback.active,
      )
    fun closeSpellcheckFromShortcut() {
      spellcheck.close()
      requestEditorFocusIfSelectionActive()
    }
    fun closeAiFeedbackFromShortcut() {
      aiFeedback.close()
      requestEditorFocusIfSelectionActive()
    }
    val screenShortcutActions =
      EditorScreenShortcutActions(
        openFindReplace = ::openFindReplace,
        closeFindReplace = ::closeFindReplaceFromShortcut,
        closeSpellcheck = ::closeSpellcheckFromShortcut,
        closeAiFeedback = ::closeAiFeedbackFromShortcut,
      )
    suspend fun openTemplateSheet() {
      val activeEditor = runtime.editor ?: return
      runtime.blur()
      focusManager.clearFocus(force = true)
      uiState.contextMenu.hide()
      sheet.present { EditorTemplateSheet(editor = activeEditor) }
    }

    LaunchedEffect(subPaneBlocksEditorInput) {
      if (subPaneBlocksEditorInput) {
        aiFeedback.close()
        spellcheck.close()
        findReplace.close()
        performInputEffects(listOf(EditorInputEffect.HideKeyboard))
      }
    }
    LaunchedEffect(aiFeedback.active) {
      if (aiFeedback.active) {
        performInputEffects(
          toolbarInputState.dispatch(ToolbarIntent.RestoreEditorInput, toolbarInputEnvironment)
        )
      }
    }
    val trustedImeBottom =
      trustedImeBottomInset(rawImeBottom = imeBottom, keyboardState = keyboardState)
    val toolbarEffectiveImeInset = effectiveImeInset(toolbarInputEnvironment)
    val imeVisible =
      isImeVisible(imeBottom = toolbarEffectiveImeInset, safeBottomInset = bottomSafeInset)
    val toolbarSuppressesSoftwareKeyboard = toolbarPanel?.let(::suppressSoftwareKeyboard) ?: false
    val toolbarTextInputSessionEnabled =
      toolbarPanel?.let {
        textInputSessionEnabledForBottomPanel(
          environment = toolbarInputEnvironment,
          imeVisible = imeVisible,
          suppressSoftwareKeyboard = toolbarSuppressesSoftwareKeyboard,
        )
      } ?: true
    val editorTextInputSessionEnabled = toolbarTextInputSessionEnabled && !subPaneBlocksEditorInput
    val editorSuppressesSoftwareKeyboard =
      toolbarSuppressesSoftwareKeyboard || subPaneBlocksEditorInput
    val previousImeVisible = remember { mutableStateOf(imeVisible) }
    val imeAppearing = !previousImeVisible.value && imeVisible
    val toolbarRetainedKeyboardInset = toolbarInputState.retainedKeyboardInset()
    val toolbarRestoreInset = toolbarInputState.keyboardRestoreInset
    val popoverOverlayState = LocalPopoverOverlayState.current
    val toolbarPresented =
      isEditorToolbarPresented(
        environment = toolbarInputEnvironment,
        activeBottomPanel = toolbarPanel?.panel,
        restoringEditorInput = toolbarRestoreInset != null,
        retainingToolbarModal = toolbarSessionState.modalActive,
      )
    val textOptionsToolbarOcclusion =
      animateDpAsState(
        targetValue =
          if (toolbarPresented && toolbarSessionState.secondaryToolbarInLayout) {
            ToolbarSecondaryStackHeight
          } else {
            0.dp
          },
        animationSpec = tween(ToolbarSecondaryVisibilityMillis),
        label = "EditorToolbarTextOptionsOcclusion",
      )
    val toolbarControlsOcclusion =
      if (toolbarPresented) {
        ToolbarHeight + ToolbarBottomPadding + textOptionsToolbarOcclusion.value
      } else {
        0.dp
      }
    val bottomPanelOrKeyboardOcclusion =
      if (toolbarPanel != null) {
        bottomSafeInset + ToolbarBottomPanelGap + toolbarPanel.height
      } else {
        toolbarRestoreInset?.let { maxOf(bottomSafeInset, it) }
          ?: maxOf(bottomSafeInset, toolbarEffectiveImeInset, toolbarRetainedKeyboardInset)
      }
    val toolbarBottomOcclusionTarget = toolbarControlsOcclusion + bottomPanelOrKeyboardOcclusion
    val inputSpaceOwnsOcclusion = !bottomPanelOpen && (imeVisible || !panelTransitionRunning)
    val toolbarBottomOcclusion =
      animateDpAsState(
        targetValue = toolbarBottomOcclusionTarget,
        animationSpec =
          if (imeAppearing) {
            snap()
          } else if (inputSpaceOwnsOcclusion) {
            snap()
          } else if (panelTransitionRunning) {
            tween(
              if (bottomPanelOpen) {
                ToolbarBottomPanelVisibilityEnterMillis
              } else {
                ToolbarBottomPanelVisibilityExitMillis
              }
            )
          } else {
            snap()
          },
        label = "EditorToolbarBottomOcclusion",
      )
    val findReplaceToolbarBottomInset =
      maxOf(bottomSafeInset, toolbarEffectiveImeInset, toolbarRetainedKeyboardInset)
    val findReplaceToolbarOcclusion =
      if (findReplaceToolbarInLayout) {
        ToolbarHeight + ToolbarBottomPadding + findReplaceToolbarBottomInset
      } else {
        0.dp
      }
    val typewriterEnabled = Preference.typewriterEnabled
    val typewriterPosition = Preference.typewriterPosition.toFloat()
    val devMode = Preference.devMode
    val displayZoom = zoomController.displayZoom
    val typewriterTargetLineHeight =
      resolveBringIntoViewTargetHeight(
        state = editorState,
        layoutSpec = layoutSpec,
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        displayZoom = displayZoom,
        density = density,
      ) ?: 0f
    val subPaneLayoutInfo = subPaneState.layoutInfo
    val subPaneBottomOcclusion = resolveSubPaneBottomOcclusion(subPaneLayoutInfo)
    val editorInputBottomOcclusion =
      if (subPaneBlocksEditorInput && subPaneLayoutInfo != null) {
        0f
      } else {
        maxOf(toolbarBottomOcclusion.value, findReplaceToolbarOcclusion).value.coerceAtLeast(0f)
      }
    val repasteAsTextVisible =
      !documentLocked &&
        uiState.focused &&
        editorState.selection != null &&
        editorState.lastHistoryTag is HistoryTag.PasteHtml
    val visibleAreas =
      screenState.resolveEditorVisibleAreas(
        topInset = topInset.value,
        rawBottomSafeInset = bottomSafeInset.value,
        rawEditorInputBottomInset = editorInputBottomOcclusion,
        rawSubPaneBottomInset = subPaneBottomOcclusion,
        overlayOcclusion =
          EditorOverlayOcclusion(
            top = maxOf(spellcheck.occlusion.top, aiFeedback.occlusion.top),
            bottom = maxOf(spellcheck.occlusion.bottom, aiFeedback.occlusion.bottom),
            bottomScrollReserve =
              maxOf(
                spellcheck.occlusion.bottomScrollReserve,
                aiFeedback.occlusion.bottomScrollReserve,
              ),
          ),
      )
    val visibleArea = visibleAreas.editor
    LaunchedEffect(imeVisible, bottomPanelOpen) { previousImeVisible.value = imeVisible }
    LaunchedEffect(layoutSpec, visibleArea.visibleBodySize.width) {
      zoomController.syncLayout(
        layoutSpec = layoutSpec,
        viewportWidth = visibleArea.visibleBodySize.width,
      )
    }
    SideEffect { uiState.updateDisplayZoom(displayZoom) }
    val pageBottomRevealSpacerHeight =
      when (layoutSpec) {
        is EditorDocumentLayoutSpec.Paginated -> visibleArea.bottomOcclusion
        is EditorDocumentLayoutSpec.Continuous -> 0f
      }
    val pagesContentHeight =
      layoutSpec.resolvePagesContentHeight(pageSizes, displayZoom, density = density)
    val bodyGeometry =
      resolveEditorBodyGeometry(
        visibleArea = visibleArea,
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        displayZoom = displayZoom,
      )
    val distanceToPagesBottom =
      if (typewriterEnabled && editor != null) {
        resolveDistanceToPagesBottom(
          state = editorState,
          layoutSpec = layoutSpec,
          uiState = uiState,
          headerHeight = screenState.headerHeight,
          pagesContentHeight = pagesContentHeight,
          target = EditorBringIntoViewTarget.CurrentSelectionHead,
          density = density,
        )
      } else {
        null
      }
    val autoScrollPolicy =
      resolveEditorAutoScrollPolicy(
        visibleArea = visibleArea,
        bottomSpacerVisibleArea = visibleAreas.bottomSpacer,
        baseBottomSpace = layoutSpec.resolveBaseBottomSpace(displayZoom),
        distanceToPagesBottom = distanceToPagesBottom,
        pageBottomRevealSpacerHeight = pageBottomRevealSpacerHeight,
        typewriterEnabled = typewriterEnabled,
        typewriterPosition = typewriterPosition,
        targetLineHeight = typewriterTargetLineHeight,
      )
    val bodyTrackWidth = bodyGeometry.pageColumnWidth.coerceAtLeast(0f)
    val isPaginatedLayout = layoutSpec is EditorDocumentLayoutSpec.Paginated
    val headerTrackWidth =
      if (isPaginatedLayout) {
          visibleArea.visibleBodySize.width
        } else {
          bodyTrackWidth
        }
        .coerceAtLeast(0f)
    val viewportScrollableState = rememberScrollable2DState { delta ->
      consumeEditorViewportTouchPan(
        viewportState = screenState.viewportState,
        deltaPx = delta,
        density = density,
        canNavigateBack = nav.canPop,
      )
    }
    val scrollFrame =
      EditorScrollFrame(
        state = editorState,
        layoutSpec = layoutSpec,
        displayZoom = displayZoom,
        visibleArea = visibleArea,
        autoScrollPolicy = autoScrollPolicy,
        headerHeight = screenState.headerHeight,
        density = density,
        editorBounds = uiState.editorBoundsInContainer,
      )
    val viewportScrollReconcileMode =
      if (
        uiState.focused &&
          screenState.sceneInForeground &&
          editor != null &&
          interactionScope.controller.interactionMode.allowsViewportScrollReconcile
      ) {
        if (subPaneLayoutInfo != null) {
          EditorViewportScrollReconcileMode.KeepVisibleAnchor
        } else if (imeAppearing) {
          EditorViewportScrollReconcileMode.RevealSelectionHead
        } else {
          EditorViewportScrollReconcileMode.KeepVisibleAnchor
        }
      } else {
        EditorViewportScrollReconcileMode.Disabled
      }
    val magnifierFocalPositionInRoot =
      interactionScope.controller.magnifierPosition?.let { position ->
        uiState.editorRectInRoot()?.let { editorRect ->
          Offset(x = editorRect.left + position.x, y = editorRect.top + position.y)
        }
      }
    SideEffect {
      val viewportZoomConfig =
        (layoutSpec as? EditorDocumentLayoutSpec.Paginated)?.let { paginatedLayoutSpec ->
          EditorViewportZoomSemanticConfig(
            layoutSpec = paginatedLayoutSpec,
            zoomController = zoomController,
            viewportState = screenState.viewportState,
            uiState = uiState,
            pageSizes = pageSizes,
            viewportWidth = visibleArea.visibleBodySize.width,
            density = density,
            onZoomSnap = { haptic.performHapticFeedback(HapticFeedbackType.SegmentTick) },
          )
        }
      interactionScope.update(
        editor = editor,
        bringIntoViewRequests = bringIntoViewRequests,
        uiState = uiState,
        visibleArea = visibleArea,
        viewportState = screenState.viewportState,
        density = density,
        scrollGestureLockState = scrollGestureLockState,
        viewportZoomConfig = viewportZoomConfig,
        pointerInputEnabled = { !popoverOverlayState.isOutsideDismissGestureActive },
        onSelectionHaptic = { haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove) },
        onRequestSoftwareKeyboard = {
          if (editorTextInputSessionEnabled && !editorSuppressesSoftwareKeyboard) {
            keyboardController?.show()
          }
        },
      )
      interactionScope.onEditorStateChanged(editorState)
      uiState.contextMenu.onEditorStateChanged(editorState)
      uiState.contextMenu.showAfterSelectionCommitIfRequested(editorState)
    }
    val paginatedLayout = layoutSpec as? EditorDocumentLayoutSpec.Paginated
    val debugWheelZoomModifier =
      if (
        PlatformModule.platform == co.typie.platform.Platform.Desktop &&
          paginatedLayout != null &&
          density > 0f
      ) {
        rememberEditorDebugWheelZoomModifier(
          state = screenState,
          onZoomSessionStart = interactionScope::beginPointerSignalZoom,
          onZoom = interactionScope::updatePointerSignalZoom,
          onZoomSessionEnd = interactionScope::endPointerSignalZoom,
        )
      } else {
        Modifier
      }

    LaunchedEffect(screenState.viewportState, viewportScrollableState) {
      snapshotFlow { viewportScrollableState.isScrollInProgress }
        .collectLatest(screenState.viewportState::updateScrollableInteractionInProgress)
    }
    LaunchedEffect(screenState.viewportState) {
      snapshotFlow { screenState.viewportState.isDirectManipulationInProgress }
        .collectLatest { inProgress ->
          if (inProgress) {
            bringIntoViewRequests.cancel()
          }
        }
    }
    LaunchedEffect(uiState.focused, screenState.sceneInForeground, editor) {
      val editorInteractionFocused =
        uiState.focused && screenState.sceneInForeground && editor != null
      if (!editorInteractionFocused) {
        uiState.contextMenu.hide()
        bringIntoViewRequests.cancel()
      }
    }

    CompositionLocalProvider(
      LocalEditorRuntime provides runtime,
      LocalEditorUiState provides uiState,
      LocalEditorExternalElementState provides externalElementState,
      LocalEditorZoomController provides zoomController,
      LocalEditorBringIntoViewRequests provides bringIntoViewRequests,
      LocalEditorInteractionScope provides interactionScope,
      LocalHazeState provides toolbarBackdropHazeState,
    ) {
      EditorScreenLayout(
        state = screenState,
        scrollFrame = scrollFrame,
        visibleArea = visibleArea,
        magnifierFocalPositionInRoot = magnifierFocalPositionInRoot,
        viewportScrollableState = viewportScrollableState,
        viewportContentWidth = bodyTrackWidth,
        viewportScrollReconcileMode = viewportScrollReconcileMode,
        onViewportWheelScroll = { uiState.contextMenu.hide() },
        onMeasuredViewportSizeChange = { viewport ->
          val editor = runtime.editor
          if (editor != null && viewport.width > 0f && viewport.height > 0f) {
            scope.launch {
              editor.await {
                enqueue(
                  Message.System(
                    SystemEvent.Resize(
                      width = viewport.width,
                      height = viewport.height,
                      scaleFactor = density.toDouble(),
                    )
                  )
                )
              }
            }
          }
        },
        header = {
          EditorHeader(
            title = model.titleDraft,
            subtitle = model.subtitleDraft,
            layoutSpec = layoutSpec,
            trackWidth = headerTrackWidth,
            loading = loading,
            topInset = topInset,
            subtitleFocusRequestVersion = subtitleFocusRequestVersion.value,
            onTitleChange = model::updateTitleDraft,
            onSubtitleChange = model::updateSubtitleDraft,
            onTitleFocused = entryState::markTitleFocused,
            onSubtitleFocused = entryState::markSubtitleFocused,
            onHeightChanged = screenState::updateHeaderHeight,
            onEnterDocument = {
              model.flushDraftsAsync()
              enterDocumentStartFromHeader(
                editor = runtime.editor,
                scope = scope,
                requestEditorFocus = ::requestEditorFocus,
              )
            },
          )
        },
        viewportOverlay = {
          EditorZoomOverlay(
            modifier =
              Modifier.align(Alignment.BottomStart)
                .padding(start = 20.dp, bottom = 20.dp + visibleArea.bottomOcclusion.dp)
          )
          EditorCharacterCountOverlay(
            editor = runtime.editor,
            viewportState = screenState.viewportState,
            visibleArea = visibleArea,
          )
        },
        overlay = {
          Box(modifier = Modifier.fillMaxSize()) {
            EditorScreenOverlayHost(
              viewportState = screenState.viewportState,
              visibleArea = visibleArea,
              autoScrollPolicy = autoScrollPolicy,
              layoutSpec = layoutSpec,
              pageSizes = pageSizes,
              displayZoom = displayZoom,
              onTableAxisActionsRequest = { target, openedSelection ->
                findReplace.close()
                aiFeedback.close()
                spellcheck.close()
                uiState.contextMenu.hide()
                subPaneState.open(
                  EditorSubPane.TableAxisActions(target = target, openedSelection = openedSelection)
                )
              },
              showDebugOverlay = devMode && model.debugViewportOverlayVisible,
              modifier = Modifier.fillMaxSize(),
            )
            SpellcheckOverlay(
              session = spellcheck,
              visibleArea = visibleAreas.base,
              modifier = Modifier.fillMaxSize(),
            )
            AiFeedbackOverlay(
              session = aiFeedback,
              visibleArea = visibleAreas.base,
              modifier = Modifier.fillMaxSize(),
            )
            val activeEditor = runtime.editor
            if (activeEditor != null) {
              EditorRepasteAsTextOverlay(
                editor = activeEditor,
                visibleArea = visibleAreas.base,
                visible = repasteAsTextVisible,
                bringIntoViewRequests = bringIntoViewRequests,
                modifier = Modifier.fillMaxSize(),
              )
            }
          }
        },
        body = {
          val graph = model.graph
          val pending by
            produceState<List<ByteArray>?>(
              initialValue = null,
              model.documentId,
              model.reloadGeneration,
            ) {
              val documentId = model.documentId
              value =
                if (documentId == null) {
                  emptyList()
                } else {
                  ChangesetDeltaStore.load(documentId).map { it.changeset }
                }
            }
          val loadedPending = pending
          if (graph != null && loadedPending != null) {
            EditorBody(
              graph = graph,
              pending = loadedPending,
              geometry = bodyGeometry,
              layoutSpec = layoutSpec,
              autoScrollPolicy = autoScrollPolicy,
              modifier = Modifier.then(debugWheelZoomModifier),
              textInputSessionEnabled = editorTextInputSessionEnabled,
              suppressSoftwareKeyboard = editorSuppressesSoftwareKeyboard,
              showDebugBodyOverlay = devMode && model.debugBodyOverlayVisible,
              showDebugSurfaceOverlay = devMode && model.debugSurfaceOverlayVisible,
              overlay = {
                if (!documentLocked) {
                  EditorDocumentPlaceholder(
                    placeholder = editorState.placeholder,
                    geometry = bodyGeometry,
                    layoutSpec = layoutSpec,
                    pageSizes = pageSizes,
                    displayZoom = displayZoom,
                    modifier = Modifier.fillMaxSize(),
                    onLoadTemplate = ::openTemplateSheet,
                  )
                }
                if (comments.virtualThreadGuardVisible) {
                  val guardInteractionSource = remember { MutableInteractionSource() }
                  Box(
                    modifier =
                      Modifier.fillMaxSize().clickable(
                        interactionSource = guardInteractionSource,
                        indication = null,
                      ) {
                        comments.requestDiscardVirtualThread()
                      }
                  )
                }
              },
            )
          }
        },
        toolbar = {
          FindReplaceToolbar(
            session = findReplace,
            visibleState = findReplaceToolbarTransition,
            bottomInset = findReplaceToolbarBottomInset,
            modifier = Modifier,
            onEscape = ::closeFindReplaceFromShortcut,
          )
          EditorToolbarHost(
            editorState = editorState,
            pagerState = toolbarPagerState,
            bottomPanelTransition = bottomPanelTransition,
            editorFocused = uiState.focused,
            inputState = toolbarInputState,
            environment = toolbarInputEnvironment,
            fontFamilies = model.toolbarFontFamilies,
            sessionState = toolbarSessionState,
            commentEnabled = comments.toolbarEnabled,
            debugOverlays =
              if (devMode) {
                EditorToolbarDebugOverlays(
                  viewportVisible = model.debugViewportOverlayVisible,
                  bodyVisible = model.debugBodyOverlayVisible,
                  surfaceVisible = model.debugSurfaceOverlayVisible,
                )
              } else {
                null
              },
            onCommentRequest = comments.requestFromTextToolbar,
            onInputEffects = ::performInputEffects,
            onToolAction = { action ->
              when (action) {
                EditorToolbarToolAction.Search -> openFindReplace()
                EditorToolbarToolAction.RelatedNotes -> {
                  findReplace.close()
                  aiFeedback.close()
                  spellcheck.close()
                  uiState.contextMenu.hide()
                  subPaneState.open(EditorSubPane.RelatedNotes)
                }
                EditorToolbarToolAction.Comment -> {
                  findReplace.close()
                  aiFeedback.close()
                  spellcheck.close()
                  comments.openFromToolPanel()
                }
                EditorToolbarToolAction.Spellcheck -> {
                  findReplace.close()
                  aiFeedback.close()
                  spellcheck.openFromToolPanel()
                  performInputEffects(
                    toolbarInputState.dispatch(
                      ToolbarIntent.RestoreEditorInput,
                      toolbarInputEnvironment,
                    )
                  )
                }
                EditorToolbarToolAction.AiFeedback -> {
                  aiFeedback.openFromToolPanel()
                }
                EditorToolbarToolAction.Timeline -> {
                  toast.show(ToastType.Notification, "타임라인 기능은 아직 준비 중이에요.")
                }
                EditorToolbarToolAction.DebugViewportOverlay -> model.toggleDebugViewportOverlay()
                EditorToolbarToolAction.DebugBodyOverlay -> model.toggleDebugBodyOverlay()
                EditorToolbarToolAction.DebugSurfaceOverlay -> model.toggleDebugSurfaceOverlay()
              }
            },
            modifier = Modifier,
          )
        },
        subPane = {
          EditorSubPaneHost(
            state = subPaneState,
            entityId = entityId,
            comments =
              CommentsSubPaneEnvironment(
                session = comments,
                myId = model.query.data.me.id,
                myRole = model.query.data.me.role,
                isOwner = entity.user.id == model.query.data.me.id,
              ),
            maxTopInset = topInset,
            safeBottomInset = bottomSafeInset,
            trustedImeBottomInset = trustedImeBottom,
            modifier = Modifier.fillMaxSize(),
          )
        },
        modifier =
          Modifier.padding(start = startInset, end = endInset).editorScreenShortcutFocusTarget(
            active = screenShortcutModeActive,
            enabled = screenState.sceneInForeground && !subPaneBlocksEditorInput,
            editorFocused = uiState.focused,
            selection = editorState.selection,
          ) { event ->
            handleEditorScreenShortcut(
              event = event,
              context = screenShortcutContext,
              actions = screenShortcutActions,
            )
          },
      )
    }
  }
}

internal fun enterDocumentStartFromHeader(
  editor: Editor?,
  scope: CoroutineScope,
  requestEditorFocus: () -> Unit,
) {
  requestEditorFocus()
  val activeEditor = editor ?: return
  scope.launch {
    activeEditor.await {
      enqueue(
        Message.Navigation(NavigationOp.Move(Movement.Document(Direction.Backward), extend = false))
      )
    }
  }
}

private val FindReplaceTopBarLeadingKey = Any()
private val FindReplaceTopBarCenterKey = Any()
private val FindReplaceTopBarTrailingKey = Any()
private val SpellcheckTopBarLeadingKey = Any()
private val SpellcheckTopBarCenterKey = Any()
private val SpellcheckTopBarTrailingKey = Any()
private val AiFeedbackTopBarLeadingKey = Any()
private val AiFeedbackTopBarCenterKey = Any()
private val AiFeedbackTopBarTrailingKey = Any()
