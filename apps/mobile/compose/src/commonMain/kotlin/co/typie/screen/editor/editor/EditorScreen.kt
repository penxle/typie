package co.typie.screen.editor.editor

import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.rememberScrollable2DState
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.EditorLocalChangesetBus
import co.typie.editor.EditorState
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.body.EditorBody
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveBaseBottomSpace
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolvePagesContentHeight
import co.typie.editor.body.toEditorDocumentLayoutSpec
import co.typie.editor.external.EditorEmbedAsset
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.Message
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
import co.typie.editor.scroll.resolveDistanceToPagesBottom
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.consumeEditorViewportTouchPan
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ime
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.PlatformModule
import co.typie.route.Route
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.screen.editor.editor.layout.EditorScreenLayout
import co.typie.screen.editor.editor.overlay.EditorScreenOverlayHost
import co.typie.screen.editor.editor.overlay.EditorZoomOverlay
import co.typie.screen.editor.editor.state.rememberEditorScreenState
import co.typie.screen.editor.editor.toolbar.EditorToolbarHost
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPadding
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelGap
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityEnterMillis
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelVisibilityExitMillis
import co.typie.screen.editor.editor.toolbar.ToolbarHeight
import co.typie.screen.editor.editor.toolbar.ToolbarInputEnvironment
import co.typie.screen.editor.editor.toolbar.effectiveImeInset
import co.typie.screen.editor.editor.toolbar.isEditorToolbarPresented
import co.typie.screen.editor.editor.toolbar.isImeVisible
import co.typie.screen.editor.editor.toolbar.rememberEditorKeyboardState
import co.typie.screen.editor.editor.toolbar.rememberEditorToolbarInputState
import co.typie.screen.editor.editor.toolbar.rememberToolbarPagerState
import co.typie.screen.editor.editor.toolbar.suppressSoftwareKeyboard
import co.typie.screen.editor.editor.toolbar.textInputSessionEnabledForBottomPanel
import co.typie.screen.editor.editor.topbar.EditorDocumentButton
import co.typie.screen.editor.editor.viewport.rememberEditorDebugWheelZoomModifier
import co.typie.storage.Preference
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.HazeState
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

@Composable
fun EditorScreen(entityId: String) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val model = viewModel { EditorViewModel(entityId) }
  val scope = rememberCoroutineScope()
  val runtime = remember(entityId) { EditorRuntime(uiScope = scope) }
  val interactionScope = remember(entityId) { EditorInteractionScope(coroutineScope = scope) }
  val uiState = remember(entityId) { EditorUiState() }
  val externalElementState = remember(entityId) { EditorExternalElementState() }
  val zoomController = rememberEditorZoomController(key = entityId)
  val screenState = rememberEditorScreenState(key = entityId)
  val loading = model.query.state !is QueryState.Success
  val entity = model.query.data.entity
  val document = entity.node.onDocument
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
  LaunchedEffect(document?.assets, loading, externalElementState) {
    if (!loading) {
      for (asset in document?.assets.orEmpty()) {
        when (asset.__typename) {
          "Image" -> {
            asset.onImage?.let { image ->
              externalElementState.images.assets[image.id] =
                EditorImageAsset(
                  id = image.id,
                  url = image.url,
                  width = image.width,
                  height = image.height,
                  ratio = image.ratio,
                  placeholder = image.placeholder,
                )
            }
          }
          "File" -> {
            asset.onFile?.let { file ->
              externalElementState.files.assets[file.id] =
                EditorFileAsset(id = file.id, name = file.name, url = file.url, size = file.size)
            }
          }
          "Embed" -> {
            asset.onEmbed?.let { embed ->
              externalElementState.embeds.assets[embed.id] =
                EditorEmbedAsset(
                  id = embed.id,
                  url = embed.url,
                  title = embed.title,
                  description = embed.description,
                  thumbnailUrl = embed.thumbnailUrl,
                  html = embed.html,
                )
            }
          }
        }
      }
    }
  }
  LaunchedEffect(runtime.error) {
    runtime.error ?: return@LaunchedEffect
    dialog.error(nav) { runtime.clearError() }
  }
  fun requestEditorFocus() {
    if (nav.current == Route.Editor(entityId)) {
      runtime.focus()
    }
  }

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
              screenState.prepareToLeaveEditorScene(
                runtime = runtime,
                uiState = uiState,
                flushDrafts = { model.flush(activeEditor) },
              )
              nav.navigate(Route.Document(entityId))
            },
            modifier = Modifier.fillMaxWidth().widthIn(max = ResponsiveContainerDefaults.MaxWidth),
          )
        }
      }
    },
    trailing = { EditorTopBarMenu(model = model) },
    scrollOffset = null,
  )

  val editor = runtime.editor
  LaunchedEffect(entityId, editor) {
    val activeEditor = editor ?: return@LaunchedEffect
    model.markBodySynced(activeEditor)
    EditorLocalChangesetBus.consume(entityId).forEach { activeEditor.receiveRemoteChangeset(it) }
  }
  LaunchedEffect(entityId, runtime) {
    EditorLocalChangesetBus.notifications(entityId).collectLatest {
      val activeEditor = runtime.editor ?: return@collectLatest
      EditorLocalChangesetBus.consume(entityId).forEach { activeEditor.receiveRemoteChangeset(it) }
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
    val editorState = editor?.state ?: EditorState.Initial
    val pageSizes = editorState.pageSizes
    val density = LocalDensity.current.density
    val haptic = LocalHapticFeedback.current
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
    val toolbarPanel = toolbarInputState.panel
    val bottomPanelOpen = toolbarPanel != null
    val bottomPanelTransition = remember { MutableTransitionState(bottomPanelOpen) }
    bottomPanelTransition.targetState = bottomPanelOpen
    val panelTransitionRunning =
      bottomPanelTransition.currentState != bottomPanelTransition.targetState
    val toolbarInputEnvironment =
      ToolbarInputEnvironment(
        visible = screenState.sceneInForeground,
        focused = uiState.focused,
        imeBottom = imeBottom,
        safeBottomInset = bottomSafeInset,
        keyboardState = keyboardState,
        panelTransitionRunning = panelTransitionRunning,
      )
    val toolbarEffectiveImeInset = effectiveImeInset(toolbarInputEnvironment)
    val imeVisible =
      isImeVisible(imeBottom = toolbarEffectiveImeInset, safeBottomInset = bottomSafeInset)
    val previousImeVisible = remember { mutableStateOf(imeVisible) }
    val imeAppearing = !previousImeVisible.value && imeVisible
    val toolbarRetainedKeyboardInset = toolbarInputState.retainedKeyboardInset()
    val toolbarRestoreInset = toolbarInputState.keyboardRestoreInset
    val toolbarPresented =
      isEditorToolbarPresented(
        environment = toolbarInputEnvironment,
        activeBottomPanel = toolbarPanel?.key,
        restoringEditorInput = toolbarRestoreInset != null,
      )
    val toolbarControlsOcclusion =
      if (toolbarPresented) {
        ToolbarHeight + ToolbarBottomPadding
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
    val typewriterEnabled = Preference.typewriterEnabled
    val typewriterPosition = Preference.typewriterPosition.toFloat()
    val devMode = Preference.devMode
    val displayZoom = zoomController.displayZoom
    val cursorLineHeight = (editorState.cursor?.line?.height ?: 0f) * displayZoom
    val visibleArea =
      screenState.resolveVisibleArea(
        topInset = topInset.value,
        rawBottomSafeInset = bottomSafeInset.value,
        rawImeInset = toolbarBottomOcclusion.value.value,
      )
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
        baseBottomSpace = layoutSpec.resolveBaseBottomSpace(displayZoom),
        distanceToPagesBottom = distanceToPagesBottom,
        pageBottomRevealSpacerHeight = pageBottomRevealSpacerHeight,
        typewriterEnabled = typewriterEnabled,
        typewriterPosition = typewriterPosition,
        cursorLineHeight = cursorLineHeight,
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
    val magnifierFocalPositionInRoot =
      interactionScope.controller.magnifierPosition?.let { position ->
        uiState.editorRectInRoot()?.let { editorRect ->
          Offset(x = editorRect.left + position.x, y = editorRect.top + position.y)
        }
      }
    val bringIntoViewRequests = rememberEditorBringIntoViewRequests()
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
        onSelectionHaptic = { haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove) },
      )
      interactionScope.onEditorStateChanged(editorState)
    }
    val toolbarSuppressesSoftwareKeyboard = toolbarPanel?.let(::suppressSoftwareKeyboard) ?: false
    val toolbarTextInputSessionEnabled =
      toolbarPanel?.let {
        textInputSessionEnabledForBottomPanel(
          environment = toolbarInputEnvironment,
          imeVisible = imeVisible,
          suppressSoftwareKeyboard = toolbarSuppressesSoftwareKeyboard,
        )
      } ?: true
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
      interactionScope.controller.onEditorFocusChanged(focused = editorInteractionFocused)
      if (!editorInteractionFocused) {
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
        viewportScrollReconcileEnabled =
          uiState.focused &&
            screenState.sceneInForeground &&
            editor != null &&
            interactionScope.controller.interactionMode.allowsViewportScrollReconcile,
        onViewportWheelScroll = interactionScope.controller::onViewportScrollStarted,
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
            onTitleChange = model::updateTitleDraft,
            onSubtitleChange = model::updateSubtitleDraft,
            onHeightChanged = screenState::updateHeaderHeight,
            onEnterDocument = {
              model.flushDraftsAsync()
              requestEditorFocus()
            },
          )
        },
        viewportOverlay = {
          EditorZoomOverlay(
            modifier =
              Modifier.align(Alignment.BottomStart)
                .padding(start = 20.dp, bottom = 20.dp + visibleArea.bottomOcclusion.dp)
          )
        },
        overlay = {
          EditorScreenOverlayHost(
            viewportState = screenState.viewportState,
            visibleArea = visibleArea,
            autoScrollPolicy = autoScrollPolicy,
            layoutSpec = layoutSpec,
            pageSizes = pageSizes,
            displayZoom = displayZoom,
            showDebugOverlay = devMode && model.debugViewportOverlayVisible,
            modifier = Modifier.fillMaxSize(),
          )
        },
        body = {
          val graph = model.graph
          if (graph != null) {
            EditorBody(
              graph = graph,
              geometry = bodyGeometry,
              layoutSpec = layoutSpec,
              autoScrollPolicy = autoScrollPolicy,
              modifier = Modifier.then(debugWheelZoomModifier),
              textInputSessionEnabled = toolbarTextInputSessionEnabled,
              suppressSoftwareKeyboard = toolbarSuppressesSoftwareKeyboard,
              showDebugBodyOverlay = devMode && model.debugBodyOverlayVisible,
              showDebugSurfaceOverlay = devMode && model.debugSurfaceOverlayVisible,
            )
          }
        },
        toolbar = {
          EditorToolbarHost(
            editorState = editorState,
            pagerState = toolbarPagerState,
            bottomPanelTransition = bottomPanelTransition,
            editorFocused = uiState.focused,
            inputState = toolbarInputState,
            environment = toolbarInputEnvironment,
            onEditorFocusRequest = ::requestEditorFocus,
            modifier = Modifier,
          )
        },
        modifier = Modifier.padding(start = startInset, end = endInset),
      )
    }
  }
}

@Composable
private fun EditorTopBarMenu(model: EditorViewModel) {
  val noop = {}

  PopoverMenu(anchor = { TopBarButton(icon = Lucide.PanelBottom) }) {
    item(icon = Lucide.Search, label = "찾기", onClick = noop)
    item(icon = Lucide.StickyNote, label = "노트", onClick = noop)
    item(icon = Lucide.MessageSquareText, label = "코멘트", onClick = noop)
    item(icon = Lucide.SpellCheck, label = "맞춤법 검사", onClick = noop)
    item(icon = Lucide.Lightbulb, label = "AI 피드백", onClick = noop)
    item(icon = Lucide.History, label = "타임라인", onClick = noop)
    if (Preference.devMode) {
      item(icon = Lucide.Send, label = "입력 로그 보내기", onClick = noop)
      divider()
      item(
        icon = Lucide.PanelTop,
        label = model.debugViewportOverlayVisible.debugToggleLabel("[디버그] 뷰포트 기준선"),
        onClick = { model.toggleDebugViewportOverlay() },
      )
      item(
        icon = Lucide.PanelBottom,
        label = model.debugBodyOverlayVisible.debugToggleLabel("[디버그] 바디 영역"),
        onClick = { model.toggleDebugBodyOverlay() },
      )
      item(
        icon = Lucide.InspectionPanel,
        label = model.debugSurfaceOverlayVisible.debugToggleLabel("[디버그] 페이지 표면"),
        onClick = { model.toggleDebugSurfaceOverlay() },
      )
    }
  }
}

private fun Boolean.debugToggleLabel(label: String): String = "$label ${if (this) "끄기" else "켜기"}"
