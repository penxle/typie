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
import co.typie.editor.scroll.resolveEditorScrollPolicy
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
    trailing = {
      TopBarButton(
        icon = if (model.isPaginatedDebugLayout) Lucide.ScrollText else Lucide.LayoutTemplate,
        onClick = { model.toggleDebugLayoutMode() },
      )
    },
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
    val displayZoom = zoomController.displayZoom
    // TODO(editor-parity): 현재는 cursor 높이만 scroll policy에 넘기고 있다. collapsed
    // selection에서는 이 값이 실제 selection head 표시 높이보다 작아서 typewriter 하단
    // 여백과 일반 cursor guard 둘 다 부족하게 계산된다. 이 높이 차이는 displayZoom과 함께
    // 같이 커지므로, 확대할수록 문서 끝에서 남는 추가 스크롤도 더 커진다. non-collapsed
    // selection도 아직 head bounds를 쓰지 못하고 있다.
    val cursorHeight = (editor?.cursor?.rect?.height ?: 0f) * displayZoom
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
    val scrollPolicy =
      resolveEditorScrollPolicy(
        visibleArea = visibleArea,
        baseBottomSpace = layoutSpec.resolveBaseBottomSpace(displayZoom),
        distanceToPagesBottom = distanceToPagesBottom,
        pageBottomRevealSpacerHeight = pageBottomRevealSpacerHeight,
        typewriterEnabled = typewriterEnabled,
        typewriterPosition = typewriterPosition,
        cursorHeight = cursorHeight,
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
        scrollPolicy = scrollPolicy,
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
                .padding(start = 20.dp, bottom = 20.dp + imeBottom)
          )
        },
        overlay = {
          EditorScreenOverlayHost(
            viewportState = screenState.viewportState,
            visibleArea = visibleArea,
            scrollPolicy = scrollPolicy,
            layoutSpec = layoutSpec,
            pageSizes = pageSizes,
            displayZoom = displayZoom,
            modifier = Modifier.fillMaxSize(),
          )
        },
        body = {
          EditorBody(
            doc = model.doc,
            selection = model.selection,
            geometry = bodyGeometry,
            layoutSpec = layoutSpec,
            scrollPolicy = scrollPolicy,
            modifier = Modifier.then(touchPinchZoomModifier).then(debugWheelZoomModifier),
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
