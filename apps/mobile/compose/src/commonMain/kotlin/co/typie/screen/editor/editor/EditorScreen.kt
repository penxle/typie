package co.typie.screen.editor.editor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.body.EditorBody
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.LocalEditorScrollController
import co.typie.editor.scroll.rememberEditorScrollController
import co.typie.ext.ime
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.screen.editor.editor.header.EditorHeader
import co.typie.screen.editor.editor.layout.EditorScreenLayout
import co.typie.screen.editor.editor.overlay.EditorScreenOverlayHost
import co.typie.screen.editor.editor.state.rememberEditorScreenState
import co.typie.screen.editor.editor.toolbar.EditorToolbarHost
import co.typie.screen.editor.editor.topbar.EditorDocumentButton
import co.typie.storage.Preference
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.flow.collectLatest

@Composable
fun EditorScreen(entityId: String) {
  val nav = Nav.current
  val model = viewModel { EditorViewModel(entityId) }
  val runtime = remember(entityId) { EditorRuntime() }
  val uiState = remember(entityId) { EditorUiState() }
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
    scrollOffset = null,
  )

  Screen(
    loadable = model.query,
    background = AppTheme.colors.surfaceDefault,
    contentPadding = PaddingValues(),
  ) { contentPadding ->
    val density = LocalDensity.current.density
    val topInset = contentPadding.calculateTopPadding()
    val bottomSafeInset = contentPadding.calculateBottomPadding()
    val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
    val typewriterEnabled = Preference.typewriterEnabled
    val typewriterPosition = Preference.typewriterPosition.toFloat()
    // TODO(editor-parity): 현재는 cursor 높이만 scroll policy에 넘기고 있다. collapsed
    // selection에서는 이 값이 실제 selection head 표시 높이보다 작아서 typewriter 하단
    // 여백과 일반 cursor guard 둘 다 몇 dp씩 모자라게 계산되고, non-collapsed selection도
    // head bounds를 쓰지 못하고 있다.
    val cursorHeight = runtime.editor?.cursor?.rect?.height ?: 0f
    val visibleArea =
      screenState.resolveVisibleArea(
        topInset = topInset.value,
        rawBottomSafeInset = bottomSafeInset.value,
        rawImeInset = imeBottom.value,
      )
    val bodyGeometry =
      resolveEditorBodyGeometry(
        visibleArea = visibleArea,
        layoutSpec = model.documentLayoutSpec,
        pageSizes = runtime.editor?.pageSizes.orEmpty(),
        typewriterEnabled = typewriterEnabled,
        typewriterPosition = typewriterPosition,
        cursorHeight = cursorHeight,
      )
    val scrollController =
      rememberEditorScrollController(
        editorProvider = { runtime.editor },
        uiState = uiState,
        scrollState = screenState.scrollState,
        visibleArea = visibleArea,
        scrollPolicy = bodyGeometry.scrollPolicy,
        headerHeight = screenState.headerHeight,
        density = density,
      )

    LaunchedEffect(scrollController, screenState.scrollState) {
      snapshotFlow { screenState.scrollState.isScrollInProgress }
        .collectLatest { inProgress ->
          if (inProgress) {
            scrollController.cancel()
          }
        }
    }
    LaunchedEffect(
      scrollController,
      uiState.focused,
      screenState.sceneInForeground,
      runtime.editor,
    ) {
      if (!uiState.focused || !screenState.sceneInForeground || runtime.editor == null) {
        scrollController.cancel()
      }
    }

    EditorScreenLayout(
      state = screenState,
      header = {
        EditorHeader(
          title = model.titleDraft,
          subtitle = model.subtitleDraft,
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
      overlay = {
        EditorScreenOverlayHost(
          visibleArea = visibleArea,
          scrollPolicy = bodyGeometry.scrollPolicy,
          modifier = Modifier.fillMaxSize(),
        )
      },
      body = {
        CompositionLocalProvider(
          LocalEditorRuntime provides runtime,
          LocalEditorUiState provides uiState,
          LocalEditorScrollController provides scrollController,
        ) {
          EditorBody(doc = model.doc, selection = model.selection, geometry = bodyGeometry)
        }
      },
      toolbar = {
        EditorToolbarHost(
          bodyFocused = screenState.shouldShowToolbar(bodyFocused = uiState.focused),
          modifier = Modifier,
          onVisibleTopChanged = screenState::updateToolbarTop,
        )
      },
      modifier = Modifier,
    )
  }
}
