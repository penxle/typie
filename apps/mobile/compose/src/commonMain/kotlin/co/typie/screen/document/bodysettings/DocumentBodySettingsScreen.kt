package co.typie.screen.document.bodysettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.DefaultRootPaginatedLayout
import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.EditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.enqueueRootLayoutMode
import co.typie.editor.enqueueRootModifier
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import co.typie.editor.preview.EditorPreview
import co.typie.editor.runtime.EditorRuntime
import co.typie.ext.imePadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.platform.PlatformModule
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.editorsettings.EditorSettingsBasicStyleSection
import co.typie.ui.component.editorsettings.EditorSettingsDetailLayoutSection
import co.typie.ui.component.editorsettings.EditorSettingsLayoutSection
import co.typie.ui.component.editorsettings.EditorSettingsSectionDivider
import co.typie.ui.component.editorsettings.EditorStyleSettings
import co.typie.ui.component.editorsettings.changedEditorModifiersFrom
import co.typie.ui.component.editorsettings.toEditorModifiers
import co.typie.ui.component.editorsettings.toEditorStyleSettings
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun DocumentBodySettingsScreen(entityId: String) {
  val model = viewModel { DocumentBodySettingsViewModel(entityId) }
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val sheet = LocalSheet.current
  val toast = LocalToast.current

  ProvideTopBar(
    center = { Text("본문 설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    loadable = model.query,
    background = AppTheme.colors.surfaceInset,
    contentPadding = PaddingValues.Zero,
  ) { contentPadding ->
    val data = (model.query.state as? QueryState.Success)?.data ?: return@Screen
    val document = data.entity.node.onDocument ?: return@Screen
    val state = document.state ?: return@Screen
    val graph = state.graph

    val colors = AppTheme.colors
    val layoutDirection = LocalLayoutDirection.current
    val topBarClearance = contentPadding.calculateTopPadding()
    val previewHeight = 200.dp
    val previewContainerHeight = topBarClearance + previewHeight
    val previewShape = RoundedCornerShape(bottomStart = AppShapes.xl, bottomEnd = AppShapes.xl)
    val graphKey = remember(graph) { graph.size to graph.contentHashCode() }
    var initial by
      remember(document.id, graphKey) { mutableStateOf<DocumentBodySettingsInitialState?>(null) }
    val settingsRuntime = remember(document.id) { EditorRuntime(uiScope = scope) }
    val previewRuntime = remember(document.id) { EditorRuntime(uiScope = scope) }
    val previewGraph = initial?.let { graph.takeIf { _ -> it.hasText } }
    val pendingPreviewChangesets =
      remember(if (previewGraph == null) null else graphKey) {
        mutableStateOf<List<ByteArray>>(emptyList())
      }
    var bodyStyle by remember(document.id) { mutableStateOf<EditorStyleSettings?>(null) }
    val editorThemeVariant = currentEditorThemeVariant()
    val editorTheme = remember(editorThemeVariant) { EditorTheme.resolve(editorThemeVariant) }
    var layout by remember(document.id) { mutableStateOf<LayoutMode?>(null) }
    val resolvedBodyStyle = bodyStyle ?: EditorStyleSettings()
    val resolvedLayout = layout ?: DefaultRootPaginatedLayout
    val controlsEnabled = initial != null && settingsRuntime.editor != null

    DisposableEffect(settingsRuntime) { onDispose { settingsRuntime.clear() } }

    LaunchedEffect(graphKey) {
      val nextInitial =
        withContext(Dispatchers.Default) {
          DocumentBodySettingsInitialState(
            hasText =
              runCatching { PlatformModule.editorHost.extractTextFromGraph(graph).isNotBlank() }
                .getOrDefault(true),
            style =
              runCatching { PlatformModule.editorHost.rootModifiersFromGraph(graph) }
                .getOrDefault(emptyList())
                .toEditorStyleSettings(),
            layout =
              runCatching { PlatformModule.editorHost.rootAttrsFromGraph(graph).layoutMode }
                .getOrDefault(DefaultRootPaginatedLayout),
          )
        }
      initial = nextInitial
      if (bodyStyle == null) bodyStyle = nextInitial.style
      if (layout == null) layout = nextInitial.layout
    }

    LaunchedEffect(settingsRuntime, graphKey, editorThemeVariant) {
      if (!settingsRuntime.canCreateEditor) return@LaunchedEffect
      settingsRuntime.attach(
        Editor.create(
          graph = graph,
          viewport = Viewport(width = 1f, height = 1f, scaleFactor = 1.0),
          scope = scope,
          themeVariant = editorThemeVariant,
          onError = { activeEditor, error -> settingsRuntime.reportError(activeEditor, error) },
        )
      )
    }

    LaunchedEffect(previewRuntime.editor, pendingPreviewChangesets.value) {
      val previewEditor = previewRuntime.editor ?: return@LaunchedEffect
      val changesets = pendingPreviewChangesets.value
      if (changesets.isEmpty()) return@LaunchedEffect
      pendingPreviewChangesets.value = emptyList()
      changesets.forEach { previewEditor.receiveRemoteChangeset(it) }
    }

    fun save(block: EditorScope.() -> Unit) {
      if (!controlsEnabled) return
      val activeEditor = settingsRuntime.editor ?: return
      scope.launch {
        model
          .updateBodySettings(editor = activeEditor, documentId = document.id, block = block)
          .withDefaultExceptionHandler(toast)
          .onOk { changesets ->
            if (changesets != null && previewGraph != null) {
              pendingPreviewChangesets.value = pendingPreviewChangesets.value + changesets
            }
          }
      }
    }

    fun saveLayout(newLayout: LayoutMode) {
      if (!controlsEnabled) return
      layout = newLayout
      save {
        enqueueRootLayoutMode(newLayout)
        enqueue(Message.System(SystemEvent.SetFocused(false)))
      }
    }

    fun saveStyle(newStyle: EditorStyleSettings) {
      if (!controlsEnabled) return
      val modifiers = newStyle.changedEditorModifiersFrom(resolvedBodyStyle)
      if (modifiers.isEmpty()) return
      bodyStyle = newStyle
      save {
        modifiers.forEach { modifier -> enqueueRootModifier(modifier) }
        enqueue(Message.System(SystemEvent.SetFocused(false)))
      }
    }

    Box(
      modifier =
        Modifier.fillMaxSize()
          .imePadding()
          .padding(
            start = contentPadding.calculateStartPadding(layoutDirection),
            end = contentPadding.calculateEndPadding(layoutDirection),
          )
    ) {
      Skeleton(enabled = !controlsEnabled, modifier = Modifier.matchParentSize()) {
        Column(
          modifier =
            Modifier.fillMaxSize()
              .verticalScroll(scrollState)
              .background(colors.surfaceDefault)
              .padding(
                top = previewContainerHeight + 12.dp,
                bottom = contentPadding.calculateBottomPadding(),
              )
              .padding(AppTheme.spacings.scrollBottomPadding)
        ) {
          EditorSettingsBasicStyleSection(
            style = resolvedBodyStyle,
            fontFamilies = model.fontFamilies,
            sheet = sheet,
            editorTheme = editorTheme,
            onStyleChange = ::saveStyle,
          )

          EditorSettingsSectionDivider()

          EditorSettingsLayoutSection(
            layout = resolvedLayout,
            sheet = sheet,
            onLayoutChange = ::saveLayout,
          )

          EditorSettingsSectionDivider()

          EditorSettingsDetailLayoutSection(style = resolvedBodyStyle, onStyleChange = ::saveStyle)
        }
      }

      Box(modifier = Modifier.fillMaxWidth()) {
        if (initial != null) {
          EditorPreview(
            layoutMode = resolvedLayout,
            runtime = previewRuntime,
            modifier = Modifier.fillMaxWidth().height(previewContainerHeight).zIndex(1f),
            shape = previewShape,
            contentTopPadding = topBarClearance,
            graph = previewGraph,
            modifiers =
              if (previewGraph == null) resolvedBodyStyle.toEditorModifiers() else emptyList(),
          )
        } else {
          Skeleton.Bone(
            modifier = Modifier.fillMaxWidth().height(previewContainerHeight).zIndex(1f),
            shape = previewShape,
          )
        }

        Box(
          modifier =
            Modifier.fillMaxWidth()
              .height(16.dp + AppShapes.xl / 2)
              .offset(y = previewContainerHeight - AppShapes.xl / 2)
              .background(
                Brush.verticalGradient(
                  colors = listOf(colors.surfaceInset, colors.surfaceInset.copy(alpha = 0f))
                )
              )
        )
      }
    }
  }
}

private data class DocumentBodySettingsInitialState(
  val hasText: Boolean,
  val style: EditorStyleSettings,
  val layout: LayoutMode,
)
