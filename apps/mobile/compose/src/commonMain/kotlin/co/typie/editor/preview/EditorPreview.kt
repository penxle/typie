package co.typie.editor.preview

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolvePaginatedPageGap
import co.typie.editor.body.toEditorDocumentLayoutSpec
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.PlainNodeEntry
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.surface.EditorPageSurface
import co.typie.editor.surface.editorPagePositionTracker
import co.typie.ext.clickable
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.collectLatest

private const val PreviewHintVisibleMillis = 850
private const val PreviewHintFadeMillis = 180

@Composable
internal fun EditorPreview(
  layoutMode: LayoutMode,
  runtime: EditorRuntime,
  modifier: Modifier = Modifier,
  shape: RoundedCornerShape,
  contentTopPadding: Dp = 0.dp,
  graph: ByteArray? = null,
  modifiers: List<EditorModifier> = emptyList(),
  hintText: String = "미리보기 텍스트",
) {
  val colors = AppTheme.colors
  val scope = rememberCoroutineScope()
  val doc = remember(layoutMode) { defaultPreviewDoc(layoutMode).withPreviewRootModifiers() }
  val sourceKey = remember(graph, doc) { graph?.let { it.size to it.contentHashCode() } ?: doc }
  val layoutSpec = remember(layoutMode) { layoutMode.toEditorDocumentLayoutSpec() }
  val background =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> colors.surfaceDefault
      is EditorDocumentLayoutSpec.Paginated -> colors.surfaceInset
    }
  val uiState = remember(sourceKey) { EditorUiState() }
  val zoomController = remember { EditorZoomController(scope = scope) }
  val bringIntoViewRequests = remember(sourceKey) { EditorBringIntoViewRequests() }
  val hintHazeState = remember { HazeState() }
  val hintShape = AppShapes.rounded(AppShapes.sm)
  val hintEvents = remember { MutableSharedFlow<Unit>(extraBufferCapacity = 1) }
  val hintAlpha = remember { Animatable(0f) }

  DisposableEffect(runtime, sourceKey) { onDispose { runtime.clear() } }
  LaunchedEffect(hintEvents) {
    hintEvents.collectLatest {
      hintAlpha.animateTo(1f, tween(PreviewHintFadeMillis))
      delay(PreviewHintVisibleMillis.toLong())
      hintAlpha.animateTo(0f, tween(PreviewHintFadeMillis))
    }
  }

  Box(modifier = modifier) {
    Box(modifier = Modifier.matchParentSize().clip(shape).hazeSource(hintHazeState)) {
      Box(modifier = Modifier.matchParentSize().background(background, shape))

      Column(modifier = Modifier.matchParentSize()) {
        if (contentTopPadding > 0.dp) {
          Spacer(modifier = Modifier.fillMaxWidth().height(contentTopPadding))
        }

        val horizontalPadding =
          when (layoutSpec) {
            is EditorDocumentLayoutSpec.Continuous -> 0.dp
            is EditorDocumentLayoutSpec.Paginated -> 20.dp
          }

        BoxWithConstraints(
          modifier =
            Modifier.weight(1f)
              .fillMaxWidth()
              .padding(horizontal = horizontalPadding)
              .clipToBounds()
        ) {
          val viewportWidth = maxWidth.value
          val viewportHeight = maxHeight.value
          if (viewportWidth <= 0f || viewportHeight <= 0f) {
            return@BoxWithConstraints
          }

          LaunchedEffect(layoutSpec, viewportWidth) {
            zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = viewportWidth)
          }
          SideEffect { uiState.updateDisplayZoom(zoomController.displayZoom) }

          CompositionLocalProvider(
            LocalEditorRuntime provides runtime,
            LocalEditorUiState provides uiState,
            LocalEditorZoomController provides zoomController,
            LocalEditorBringIntoViewRequests provides bringIntoViewRequests,
          ) {
            EditorPreviewContent(
              graph = graph,
              doc = doc,
              sourceKey = sourceKey,
              modifiers = modifiers,
              editorScope = scope,
              runtime = runtime,
              uiState = uiState,
              zoomController = zoomController,
              layoutSpec = layoutSpec,
              viewportWidth = viewportWidth,
              viewportHeight = viewportHeight,
            )
          }
        }
      }
    }

    Column(modifier = Modifier.matchParentSize().clip(shape)) {
      if (contentTopPadding > 0.dp) {
        Spacer(modifier = Modifier.fillMaxWidth().height(contentTopPadding))
      }

      Box(
        modifier = Modifier.weight(1f).fillMaxWidth().clickable { hintEvents.tryEmit(Unit) },
        contentAlignment = Alignment.Center,
      ) {
        if (hintAlpha.value > 0f) {
          Box(
            modifier =
              Modifier.graphicsLayer { alpha = hintAlpha.value }
                .clip(hintShape)
                .hazeEffect(hintHazeState) {
                  backgroundColor = colors.surfaceInset
                  blurRadius = 6.dp
                }
                .background(colors.surfaceInset.copy(alpha = 0.36f), hintShape)
                .padding(horizontal = 10.dp, vertical = 5.dp)
          ) {
            Text(text = hintText, style = AppTheme.typography.caption, color = colors.textMuted)
          }
        }
      }
    }
  }
}

@Composable
private fun EditorPreviewContent(
  graph: ByteArray?,
  doc: PlainDoc,
  sourceKey: Any,
  modifiers: List<EditorModifier>,
  editorScope: CoroutineScope,
  runtime: EditorRuntime,
  uiState: EditorUiState,
  zoomController: EditorZoomController,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
  viewportHeight: Float,
) {
  val density = LocalDensity.current
  val themeVariant = currentEditorThemeVariant()
  val displayZoom = zoomController.displayZoom
  val editor = runtime.editor

  LaunchedEffect(editor, modifiers) {
    if (editor == null) {
      return@LaunchedEffect
    }

    editor.await {
      if (modifiers.isNotEmpty()) {
        enqueue(Message.Selection(SelectionOp.All))
        modifiers.forEach { modifier -> enqueue(Message.Modifier(ModifierOp.Set(modifier))) }
      }
      enqueue(Message.Selection(SelectionOp.SetFlat(start = 0, end = 0)))
      enqueue(Message.System(SystemEvent.SetFocused(false)))
    }
  }

  LaunchedEffect(
    sourceKey,
    viewportWidth,
    viewportHeight,
    density.density,
    themeVariant,
    modifiers,
  ) {
    val viewport =
      Viewport(
        width = viewportWidth,
        height = viewportHeight,
        scaleFactor = density.density.toDouble(),
      )
    val activeEditor = runtime.editor
    if (activeEditor == null) {
      val nextEditor =
        if (graph != null) {
          Editor.create(
            graph = graph,
            viewport = viewport,
            themeVariant = themeVariant,
            scope = editorScope,
            onError = { activeEditor, error -> runtime.reportError(activeEditor, error) },
          )
        } else {
          Editor.createFromDoc(
            doc = doc.withPreviewRootModifiers(modifiers),
            viewport = viewport,
            themeVariant = themeVariant,
            scope = editorScope,
            onError = { activeEditor, error -> runtime.reportError(activeEditor, error) },
          )
        }
      runtime.attach(nextEditor)
    } else {
      activeEditor.await {
        enqueue(Message.System(SystemEvent.SetThemeVariant(themeVariant)))
        enqueue(
          Message.System(
            SystemEvent.Resize(
              width = viewport.width,
              height = viewport.height,
              scaleFactor = viewport.scaleFactor,
            )
          )
        )
        enqueue(Message.System(SystemEvent.SetFocused(false)))
      }
    }
  }

  val pageSizes = editor?.pageSizes.orEmpty()
  val pageSpacing =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> 0.dp
      is EditorDocumentLayoutSpec.Paginated -> resolvePaginatedPageGap(displayZoom).dp
    }
  val pageBackground =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> AppTheme.colors.surfaceDefault
      is EditorDocumentLayoutSpec.Paginated -> Color.Transparent
    }
  EditorPreviewPageStack(
    pageSpacing = pageSpacing,
    modifier = Modifier.fillMaxSize().clipToBounds().background(pageBackground),
  ) {
    pageSizes.forEachIndexed { index, size ->
      EditorPageSurface(
        page = index,
        width = size.width,
        height = size.height,
        showChrome = layoutSpec is EditorDocumentLayoutSpec.Paginated,
        debugBottomMarginHeight =
          when (layoutSpec) {
            is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageMarginBottom
            is EditorDocumentLayoutSpec.Continuous -> 0f
          },
        modifier =
          Modifier.editorPagePositionTracker(
            uiState = uiState,
            page = index,
            density = density.density,
          ),
      )
    }
  }
}

@Composable
private fun EditorPreviewPageStack(
  pageSpacing: Dp,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  Layout(content = content, modifier = modifier) { measurables, constraints ->
    val placeables = measurables.map { measurable ->
      measurable.measure(
        Constraints(
          minWidth = 0,
          maxWidth = Constraints.Infinity,
          minHeight = 0,
          maxHeight = Constraints.Infinity,
        )
      )
    }
    val spacingPx = pageSpacing.roundToPx()

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      var y = 0
      placeables.forEach { placeable ->
        val x = (constraints.maxWidth - placeable.width) / 2
        placeable.placeRelative(x = x, y = y)
        y += placeable.height + spacingPx
      }
    }
  }
}

private fun defaultPreviewDoc(layoutMode: LayoutMode): PlainDoc {
  val paragraphIds = DefaultPreviewParagraphs.indices.map { index -> "${index * 2 + 1}" }
  val textIds = DefaultPreviewParagraphs.indices.map { index -> "${index * 2 + 2}" }
  val nodes = buildMap {
    put(
      "0",
      PlainNodeEntry(
        parent = null,
        children = paragraphIds,
        modifiers = DefaultPreviewRootModifiers,
        node = PlainNode.Root(layoutMode = layoutMode),
      ),
    )

    DefaultPreviewParagraphs.forEachIndexed { index, paragraph ->
      put(
        paragraphIds[index],
        PlainNodeEntry(
          parent = "0",
          children = listOf(textIds[index]),
          modifiers = emptyMap(),
          node = PlainNode.Paragraph,
        ),
      )
      put(
        textIds[index],
        PlainNodeEntry(
          parent = paragraphIds[index],
          children = emptyList(),
          modifiers = emptyMap(),
          node = PlainNode.Text(text = paragraph),
        ),
      )
    }
  }
  return PlainDoc(nodes = nodes)
}

private fun PlainDoc.withPreviewRootModifiers(): PlainDoc {
  val rootEntry = nodes["0"] ?: return this
  val rootModifiers = DefaultPreviewRootModifiers + rootEntry.modifiers
  return PlainDoc(
    nodes =
      nodes +
        ("0" to
          PlainNodeEntry(
            parent = rootEntry.parent,
            children = rootEntry.children,
            modifiers = rootModifiers,
            node = rootEntry.node,
          ))
  )
}

private fun PlainDoc.withPreviewRootModifiers(modifiers: List<EditorModifier>): PlainDoc {
  val rootEntry = nodes["0"] ?: return this
  val rootModifiers =
    DefaultPreviewRootModifiers + rootEntry.modifiers + modifiers.toPreviewRootModifierMap()
  return PlainDoc(
    nodes =
      nodes +
        ("0" to
          PlainNodeEntry(
            parent = rootEntry.parent,
            children = rootEntry.children,
            modifiers = rootModifiers,
            node = rootEntry.node,
          ))
  )
}

private fun List<EditorModifier>.toPreviewRootModifierMap(): Map<ModifierType, EditorModifier> =
  mapNotNull { modifier ->
      val type =
        when (modifier) {
          is EditorModifier.FontFamily -> ModifierType.FontFamily
          is EditorModifier.FontSize -> ModifierType.FontSize
          is EditorModifier.FontWeight -> ModifierType.FontWeight
          is EditorModifier.TextColor -> ModifierType.TextColor
          is EditorModifier.BackgroundColor -> ModifierType.BackgroundColor
          is EditorModifier.LetterSpacing -> ModifierType.LetterSpacing
          is EditorModifier.LineHeight -> ModifierType.LineHeight
          is EditorModifier.ParagraphIndent -> ModifierType.ParagraphIndent
          is EditorModifier.BlockGap -> ModifierType.BlockGap
          else -> null
        }
      type?.let { it to modifier }
    }
    .toMap()

private val DefaultPreviewRootModifiers =
  mapOf(
    ModifierType.FontFamily to EditorModifier.FontFamily("Pretendard"),
    ModifierType.FontSize to EditorModifier.FontSize(1200),
    ModifierType.FontWeight to EditorModifier.FontWeight(400),
    ModifierType.TextColor to EditorModifier.TextColor("black"),
    ModifierType.BackgroundColor to EditorModifier.BackgroundColor("none"),
    ModifierType.LetterSpacing to EditorModifier.LetterSpacing(0),
    ModifierType.LineHeight to EditorModifier.LineHeight(160),
    ModifierType.ParagraphIndent to EditorModifier.ParagraphIndent(100),
    ModifierType.BlockGap to EditorModifier.BlockGap(100),
  )

private val DefaultPreviewParagraphs =
  listOf(
    "우리는 종종 완성된 글보다, 쓰기 시작한 마음을 더 오래 기억합니다. " +
      "아직 다듬어지지 않은 문장에도 그날의 온도와 망설임, 끝내 붙잡고 싶었던 생각이 남아 있습니다.",
    "좋은 글쓰기 환경은 글보다 앞서 나서지 않습니다. " +
      "눈에 편안한 여백, 오래 바라봐도 피로하지 않은 글자, " +
      "생각의 속도를 방해하지 않는 간격만으로 충분합니다. " +
      "조용히 곁을 지키는 설정은 쓰는 사람이 자기 목소리에 조금 더 가까이 다가가도록 돕습니다.",
    "오늘의 기록이 아주 작은 메모로 시작하더라도 괜찮습니다. " +
      "한 줄의 문장은 다음 문장을 부르고, " +
      "오래 머문 생각은 언젠가 누군가에게 닿을 수 있는 글이 됩니다.",
    "\"계속 쓰는 사람에게는, 아직 쓰지 않은 문장이 늘 남아 있습니다.\"",
  )
