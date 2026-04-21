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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.body.EditorBody
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
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
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.theme.AppTheme

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
      // TODO(editor-parity): Flush header drafts on app background/inactive transitions once
      // the editor screen lifecycle is wired beyond composition disposal.
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
    val topInset = contentPadding.calculateTopPadding()
    val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
    val geometry =
      screenState.resolveBodyGeometry(
        topInset = topInset.value,
        rawImeInset = imeBottom.value,
        layoutSpec = model.documentLayoutSpec,
        pageSizes = runtime.editor?.pageSizes.orEmpty(),
      )

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
      body = {
        CompositionLocalProvider(
          LocalEditorRuntime provides runtime,
          LocalEditorUiState provides uiState,
        ) {
          EditorBody(
            doc = model.doc,
            selection = model.selection,
            geometry = geometry,
            overlay = { EditorScreenOverlayHost(modifier = Modifier.fillMaxSize()) },
          )
        }
      },
      toolbar = {
        EditorToolbarHost(
          bodyFocused = screenState.shouldShowToolbar(bodyFocused = uiState.focused),
          modifier = Modifier,
          onHeightChanged = screenState::updateToolbarHeight,
        )
      },
      modifier = Modifier,
    )
  }
}
