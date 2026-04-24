package co.typie.screen.editor.editor

import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.body.EditorBody
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveBaseBottomSpace
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolvePagesContentHeight
import co.typie.editor.rememberEditorZoomController
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorScrollTarget
import co.typie.editor.scroll.LocalEditorAutoScrollController
import co.typie.editor.scroll.rememberEditorAutoScrollController
import co.typie.editor.scroll.resolveDistanceToPagesBottom
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.consumeEditorViewportTouchPan
import co.typie.ext.ime
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.PlatformModule
import co.typie.route.Route
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.screen.editor.editor.header.resolvePaginatedHeaderTrackWidth
import co.typie.screen.editor.editor.layout.EditorScreenLayout
import co.typie.screen.editor.editor.overlay.EditorScreenOverlayHost
import co.typie.screen.editor.editor.overlay.EditorZoomOverlay
import co.typie.screen.editor.editor.state.rememberEditorScreenState
import co.typie.screen.editor.editor.toolbar.EditorToolbarHost
import co.typie.screen.editor.editor.topbar.EditorDocumentButton
import co.typie.screen.editor.editor.viewport.rememberEditorDebugWheelZoomModifier
import co.typie.screen.editor.editor.viewport.rememberEditorTouchPinchZoomModifier
import co.typie.storage.Preference
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.flow.collectLatest

@Composable
fun EditorScreen(entityId: String) {
  val nav = Nav.current
  val model = viewModel { EditorViewModel(entityId) }
  val runtime = remember(entityId) { EditorRuntime() }
  val uiState = remember(entityId) { EditorUiState() }
  val zoomController = rememberEditorZoomController(key = entityId)
  val screenState = rememberEditorScreenState(key = entityId)
  val loading = model.query.state !is QueryState.Success
  val entity = model.query.data.entity
  val document = entity.node.onDocument
  DisposableEffect(model) {
    onDispose {
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
              screenState.prepareToLeaveEditorScene(
                runtime = runtime,
                uiState = uiState,
                flushDrafts = model::flushDrafts,
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

  Screen(
    loadable = model.query,
    background = AppTheme.colors.surfaceDefault,
    contentPadding = PaddingValues(),
  ) { contentPadding ->
    val layoutSpec = model.documentLayoutSpec
    val editor = runtime.editor
    val pageSizes = editor?.pageSizes.orEmpty()
    val density = LocalDensity.current.density
    val topInset = contentPadding.calculateTopPadding()
    val bottomSafeInset = contentPadding.calculateBottomPadding()
    val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
    val typewriterEnabled = Preference.typewriterEnabled
    val typewriterPosition = Preference.typewriterPosition.toFloat()
    val devMode = Preference.devMode
    val displayZoom = zoomController.displayZoom
    val cursorLineHeight = (editor?.cursor?.line?.height ?: 0f) * displayZoom
    val visibleArea =
      screenState.resolveVisibleArea(
        topInset = topInset.value,
        rawBottomSafeInset = bottomSafeInset.value,
        rawImeInset = imeBottom.value,
      )
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
    val pagesContentHeight = layoutSpec.resolvePagesContentHeight(pageSizes, displayZoom)
    val distanceToPagesBottom =
      if (typewriterEnabled && editor != null) {
        resolveDistanceToPagesBottom(
          editor = editor,
          uiState = uiState,
          headerHeight = screenState.headerHeight,
          pagesContentHeight = pagesContentHeight,
          bottomOcclusion = visibleArea.bottomOcclusion,
          target = EditorScrollTarget.CurrentSelectionHead,
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
    val bodyGeometry =
      resolveEditorBodyGeometry(
        visibleArea = visibleArea,
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        displayZoom = displayZoom,
      )
    val headerTrackWidth =
      when (layoutSpec) {
        is EditorDocumentLayoutSpec.Paginated ->
          resolvePaginatedHeaderTrackWidth(
            trackWidth = bodyGeometry.pageColumnWidth,
            displayZoom = displayZoom,
          )
        is EditorDocumentLayoutSpec.Continuous -> bodyGeometry.pageColumnWidth
      }
    val viewportScrollableState = rememberScrollable2DState { delta ->
      consumeEditorViewportTouchPan(
        viewportState = screenState.viewportState,
        deltaPx = delta,
        density = density,
      )
    }
    val autoScrollController =
      rememberEditorAutoScrollController(
        editorProvider = { runtime.editor },
        uiState = uiState,
        viewportState = screenState.viewportState,
        isDirectScrollInProgress = { screenState.viewportState.isDirectManipulationInProgress },
        visibleArea = visibleArea,
        autoScrollPolicy = autoScrollPolicy,
        headerHeight = screenState.headerHeight,
      )
    val paginatedLayout = layoutSpec as? EditorDocumentLayoutSpec.Paginated
    val touchPinchZoomModifier =
      if (paginatedLayout != null && density > 0f) {
        rememberEditorTouchPinchZoomModifier(
          state = screenState,
          layoutSpec = paginatedLayout,
          zoomController = zoomController,
          uiState = uiState,
          pageSizes = pageSizes,
          density = density,
        )
      } else {
        Modifier
      }
    val debugWheelZoomModifier =
      if (
        PlatformModule.platform == co.typie.platform.Platform.Desktop &&
          paginatedLayout != null &&
          density > 0f
      ) {
        rememberEditorDebugWheelZoomModifier(
          state = screenState,
          layoutSpec = paginatedLayout,
          zoomController = zoomController,
          uiState = uiState,
          pageSizes = pageSizes,
          density = density,
        )
      } else {
        Modifier
      }

    LaunchedEffect(screenState.viewportState, viewportScrollableState) {
      snapshotFlow { viewportScrollableState.isScrollInProgress }
        .collectLatest(screenState.viewportState::updateScrollableInteractionInProgress)
    }
    LaunchedEffect(autoScrollController, screenState.viewportState) {
      snapshotFlow { screenState.viewportState.isDirectManipulationInProgress }
        .collectLatest { inProgress ->
          if (inProgress) {
            autoScrollController.cancel()
          }
        }
    }
    LaunchedEffect(autoScrollController, uiState.focused, screenState.sceneInForeground, editor) {
      if (!uiState.focused || !screenState.sceneInForeground || editor == null) {
        autoScrollController.cancel()
      }
    }

    CompositionLocalProvider(
      LocalEditorRuntime provides runtime,
      LocalEditorUiState provides uiState,
      LocalEditorZoomController provides zoomController,
      LocalEditorAutoScrollController provides autoScrollController,
    ) {
      EditorScreenLayout(
        state = screenState,
        viewportScrollableState = viewportScrollableState,
        viewportContentWidth = headerTrackWidth,
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
              runtime.focus()
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
          EditorBody(
            doc = model.doc,
            selection = model.selection,
            geometry = bodyGeometry,
            layoutSpec = layoutSpec,
            autoScrollPolicy = autoScrollPolicy,
            modifier = Modifier.then(touchPinchZoomModifier).then(debugWheelZoomModifier),
            showDebugBodyOverlay = devMode && model.debugBodyOverlayVisible,
            showDebugSurfaceOverlay = devMode && model.debugSurfaceOverlayVisible,
          )
        },
        toolbar = {
          EditorToolbarHost(
            bodyFocused = screenState.shouldShowToolbar(bodyFocused = uiState.focused),
            modifier = Modifier,
          )
        },
        modifier = Modifier,
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
    item(icon = Lucide.Settings, label = "본문 설정", onClick = noop)
    if (Preference.devMode) {
      item(icon = Lucide.Send, label = "입력 로그 보내기", onClick = noop)
      divider()
      item(
        icon = if (model.isPaginatedDebugLayout) Lucide.ScrollText else Lucide.LayoutTemplate,
        label = "[디버그] 레이아웃 토글",
        onClick = { model.toggleDebugLayoutMode() },
      )
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
