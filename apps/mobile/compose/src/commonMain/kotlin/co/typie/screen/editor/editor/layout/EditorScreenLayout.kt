package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.MutatePriority
import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.foundation.gestures.ScrollableDefaults
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.layer.GraphicsLayer
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.node.CompositionLocalConsumerModifierNode
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.node.currentValueOf
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.PublishedBundle
import co.typie.editor.SurfacePageSpan
import co.typie.editor.VerticalSpan
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolveMeasuredPageLength
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.body.resolvePagesContentHeight
import co.typie.editor.ext.unclippedBoundsInRoot
import co.typie.editor.interaction.EditorPlatformIndirectScaleBridge
import co.typie.editor.interaction.EditorScreenPointerSequence
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.interaction.editorPlatformIndirectScale
import co.typie.editor.interaction.isDirectDown
import co.typie.editor.interaction.observeEditorScreenPointerSequence
import co.typie.editor.requiredSurfacePages
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorScrollIntentResult
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.isEditorScrollTargetVisible
import co.typie.editor.scroll.resolveEditorScrollIntent
import co.typie.editor.scroll.resolveInstantRevealPreparationViewports
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockHandle
import co.typie.navigation.LocalNavigationPopNestedScroll
import co.typie.navigation.NavigationForeground
import co.typie.navigation.navigationPopNestedScroll
import co.typie.platform.isTouchDragPointer
import co.typie.screen.editor.editor.overlay.EditorMagnifierPlacement
import co.typie.screen.editor.editor.overlay.editorNativeMagnifier
import co.typie.screen.editor.editor.overlay.editorSoftwareMagnifierLens
import co.typie.screen.editor.editor.overlay.editorSoftwareMagnifierSource
import co.typie.screen.editor.editor.overlay.resolveEditorMagnifierPlacement
import co.typie.screen.editor.editor.state.EditorScreenState
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource
import kotlin.coroutines.coroutineContext
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

private enum class EditorScreenLayoutSlot {
  ViewportContent,
  ViewportSurfaceOverlay,
  ViewportOverlay,
  Overlay,
  Toolbar,
  SubPane,
}

private const val EditorViewportScrollAnchorTolerance = 1f

private object EditorViewportNestedScrollConnection : NestedScrollConnection

private data class EditorSurfacePreparation(
  val requiredPages: Set<Int>,
  val scrollIntent: EditorScrollIntentResult?,
  val maximumScrollY: Float,
)

private data object SharePointerInputWithSiblingsElement :
  ModifierNodeElement<SharePointerInputWithSiblingsNode>() {
  override fun create(): SharePointerInputWithSiblingsNode = SharePointerInputWithSiblingsNode()

  override fun update(node: SharePointerInputWithSiblingsNode) = Unit
}

private class SharePointerInputWithSiblingsNode : Modifier.Node(), PointerInputModifierNode {
  override fun sharePointerInputWithSiblings(): Boolean = true

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) =
    Unit

  override fun onCancelPointerInput() = Unit
}

private data object ViewportDirectControlElement :
  ModifierNodeElement<ViewportDirectControlNode>() {
  override fun create(): ViewportDirectControlNode = ViewportDirectControlNode()

  override fun update(node: ViewportDirectControlNode) = Unit
}

private class ViewportDirectControlNode :
  Modifier.Node(), PointerInputModifierNode, CompositionLocalConsumerModifierNode {
  private var scrollGestureLockHandle: ScrollGestureLockHandle? = null

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (pass != PointerEventPass.Initial) {
      return
    }
    pointerEvent.changes.forEach { change ->
      if (change.isDirectDown(pointerEvent)) {
        if (
          scrollGestureLockHandle == null &&
            change.type == PointerType.Mouse &&
            change.type.isTouchDragPointer()
        ) {
          scrollGestureLockHandle = currentValueOf(LocalScrollGestureLockState).acquire()
        }
        change.consume()
      } else if (change.changedToUpIgnoreConsumed()) {
        releaseScrollGestureLock()
      }
    }
  }

  override fun onCancelPointerInput() {
    releaseScrollGestureLock()
  }

  override fun onDetach() {
    releaseScrollGestureLock()
    super.onDetach()
  }

  private fun releaseScrollGestureLock() {
    scrollGestureLockHandle?.release()
    scrollGestureLockHandle = null
  }
}

private fun Modifier.sharePointerInputWithSiblings(): Modifier =
  this then SharePointerInputWithSiblingsElement

internal fun Modifier.viewportDirectControl(): Modifier = this then ViewportDirectControlElement

internal enum class EditorViewportScrollReconcileMode {
  Disabled,
  KeepVisibleAnchor,
  RevealSelectionHead,
}

@Composable
internal fun EditorScreenLayout(
  state: EditorScreenState,
  editor: Editor? = null,
  scrollFrame: EditorScrollFrame,
  visibleArea: EditorVisibleArea,
  magnifierFocalPositionInRoot: Offset? = null,
  viewportScrollableState: Scrollable2DState,
  viewportContentWidth: Float,
  isCurrentNavigationRoute: Boolean = true,
  editorInteractionEnabled: Boolean = true,
  platformIndirectScaleEnabled: Boolean = editorInteractionEnabled,
  viewportScrollReconcileMode: EditorViewportScrollReconcileMode,
  onViewportIndirectInput: () -> Unit = {},
  onRequestEditing: (() -> Boolean)? = null,
  onMeasuredViewportSizeChange: (Size) -> Unit,
  header: @Composable () -> Unit,
  body: @Composable (PublishedBundle?) -> Unit,
  viewportSurfaceOverlay: @Composable BoxScope.() -> Unit = {},
  viewportOverlay: @Composable BoxScope.() -> Unit = {},
  overlay: @Composable () -> Unit = {},
  toolbar: @Composable () -> Unit,
  subPane: @Composable BoxScope.() -> Unit = {},
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val platformIndirectScaleBridge = remember { EditorPlatformIndirectScaleBridge() }
  val viewConfiguration = LocalViewConfiguration.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val appliedState = editor?.appliedState
  val bringIntoViewRequest = appliedState?.let {
    bringIntoViewRequests.activateForVersion(it.version)
  }
  if (appliedState != null && bringIntoViewRequest == null) {
    SideEffect { bringIntoViewRequests.discardObsoleteForVersion(appliedState.version) }
  }
  val surfacePreparation = editor?.let {
    resolveEditorSurfacePreparation(
      editor = it,
      scrollFrame = scrollFrame.copy(state = it.appliedState),
      currentScroll = state.viewportState.scrollOffset.y,
      bringIntoViewRequest = bringIntoViewRequest,
    )
  }
  if (editor != null && surfacePreparation != null) {
    SideEffect { editor.requestSurfacePages(surfacePreparation.requiredPages) }
  }
  val interactionScope = LocalEditorInteractionScope.current
  val uiState = LocalEditorUiState.current
  val toolbarBackdropHazeState = LocalHazeState.current
  val navigationPopNestedScroll = LocalNavigationPopNestedScroll.current
  val viewportNestedScrollDispatcher = remember { NestedScrollDispatcher() }
  val screenPointerSequence = remember { EditorScreenPointerSequence() }
  val viewportFlingBehavior = ScrollableDefaults.flingBehavior()
  val scrollReconcileState = remember { EditorViewportScrollReconcileState() }
  val coroutineScope = rememberCoroutineScope()
  var smoothScrollJob by remember { mutableStateOf<Job?>(null) }
  var layoutBoundsInRoot by remember { mutableStateOf<Rect?>(null) }
  if (isCurrentNavigationRoute) {
    DisposableEffect(navigationPopNestedScroll, viewportScrollableState) {
      navigationPopNestedScroll?.registerScrollInterruption(
        owner = viewportScrollableState,
        isScrollInProgress = { viewportScrollableState.isScrollInProgress },
        interrupt = {
          coroutineScope.launch(start = CoroutineStart.UNDISPATCHED) {
            viewportScrollableState.scroll(MutatePriority.UserInput) {}
          }
        },
      )
      onDispose { navigationPopNestedScroll?.unregisterScrollInterruption(viewportScrollableState) }
    }
  }
  LaunchedEffect(
    state.viewportState.isTransforming,
    state.viewportState.isDirectManipulationInProgress,
  ) {
    if (state.viewportState.isTransforming || state.viewportState.isDirectManipulationInProgress) {
      smoothScrollJob?.cancel()
      smoothScrollJob = null
    }
  }
  val magnifierPlacement = layoutBoundsInRoot?.let { bounds ->
    val focalPositionInRoot = magnifierFocalPositionInRoot ?: return@let null
    resolveEditorMagnifierPlacement(
      focalPosition =
        Offset(x = focalPositionInRoot.x - bounds.left, y = focalPositionInRoot.y - bounds.top),
      overlaySize = bounds.size,
      visibleArea = visibleArea,
      density = density.density,
    )
  }
  val resolveSize: (Int, Int) -> Size =
    remember(density) {
      { width, height -> Size(width = width / density.density, height = height / density.density) }
    }
  val softwareMagnifierSource = rememberGraphicsLayer()
  val editorInteractionModifier =
    Modifier.editorInteractions(
        interactionController = interactionScope.controller,
        geometry = interactionScope,
        screenPointerSequence = screenPointerSequence,
        platformIndirectScaleBridge = platformIndirectScaleBridge,
        scrollGestureLockState = scrollGestureLockState,
        scrollableState = viewportScrollableState,
        nestedScrollDispatcher = viewportNestedScrollDispatcher,
        flingBehavior = viewportFlingBehavior,
        touchSlop = viewConfiguration.touchSlop,
        maximumFlingVelocity = viewConfiguration.maximumFlingVelocity,
        density = density.density,
        enabled = editorInteractionEnabled,
        onViewportIndirectInput = onViewportIndirectInput,
        onNestedScrollCancel = { navigationPopNestedScroll?.cancel() },
      )
      .editorPlatformIndirectScale(
        bridge = platformIndirectScaleBridge,
        enabled = platformIndirectScaleEnabled,
        density = density.density,
      )
  val readingEditSemanticsModifier =
    if (onRequestEditing == null) {
      Modifier
    } else {
      Modifier.semantics {
        customActions = listOf(CustomAccessibilityAction(label = "편집") { onRequestEditing() })
      }
    }

  Box(
    modifier =
      modifier
        .fillMaxSize()
        .observeEditorScreenPointerSequence(screenPointerSequence)
        .editorNativeMagnifier(magnifierPlacement)
        .onGloballyPositioned { coordinates ->
          layoutBoundsInRoot = coordinates.unclippedBoundsInRoot()
        }
  ) {
    SubcomposeLayout(
      modifier =
        Modifier.fillMaxSize()
          .editorSoftwareMagnifierSource(
            sourceLayer = softwareMagnifierSource,
            active = magnifierPlacement != null,
          )
    ) { constraints ->
      editor?.publicationVersion
      val viewportWidth = constraints.maxWidth / density.density
      val measuredViewportSize = resolveSize(constraints.maxWidth, constraints.maxHeight)
      val measuredVisibleArea = scrollFrame.visibleArea.copy(viewport = measuredViewportSize)
      val measuredScrollFrame = scrollFrame.copy(visibleArea = measuredVisibleArea)
      val resolvedContentWidth =
        resolveEditorViewportContentWidth(
          viewportWidth = viewportWidth,
          contentTrackWidth = viewportContentWidth,
        )
      val viewportHeight = constraints.maxHeight
      val viewportConstraints =
        constraints.copy(
          minWidth = constraints.maxWidth,
          maxWidth = constraints.maxWidth,
          minHeight = viewportHeight,
          maxHeight = viewportHeight,
        )
      val publishedBundle = editor?.publishedBundle
      val currentAppliedState = editor?.appliedState
      val currentRequest = currentAppliedState?.let {
        bringIntoViewRequests.activateForVersion(it.version)
      }
      val currentPreparation =
        if (editor != null && currentAppliedState != null) {
          resolveEditorSurfacePreparation(
            editor = editor,
            scrollFrame = measuredScrollFrame.copy(state = currentAppliedState),
            currentScroll = state.viewportState.scrollOffset.y,
            bringIntoViewRequest = currentRequest,
          )
        } else {
          null
        }
      val candidateBundle = currentPreparation?.let {
        editor?.publishIfReady(requiredPages = it.requiredPages)
      }
      val acceptedBundle = candidateBundle?.takeIf { candidate ->
        val acceptingEditor = editor ?: return@takeIf false
        val latestState = acceptingEditor.appliedState
        val latestRequest = bringIntoViewRequests.activateForVersion(latestState.version)
        latestState === candidate.snapshot &&
          latestRequest === currentRequest &&
          currentPreparation ==
            resolveEditorSurfacePreparation(
              editor = acceptingEditor,
              scrollFrame = measuredScrollFrame.copy(state = latestState),
              currentScroll = state.viewportState.scrollOffset.y,
              bringIntoViewRequest = latestRequest,
            ) &&
          acceptingEditor.acceptPublication(candidate)
      }
      val placedBundle = acceptedBundle ?: publishedBundle
      val acceptedRequest = currentRequest.takeIf { acceptedBundle != null }
      var requestToMarkPresented: EditorBringIntoViewRequests.Request? = null
      val viewportContentPlaceables =
        subcompose(EditorScreenLayoutSlot.ViewportContent) {
            Layout(
              modifier =
                Modifier.fillMaxSize()
                  .clipToBounds()
                  .hazeSource(toolbarBackdropHazeState)
                  .navigationPopNestedScroll()
                  .nestedScroll(
                    EditorViewportNestedScrollConnection,
                    viewportNestedScrollDispatcher,
                  )
                  .then(readingEditSemanticsModifier)
                  .then(editorInteractionModifier),
              content = {
                Column {
                  Box(modifier = Modifier.fillMaxWidth()) { header() }
                  Box(
                    modifier =
                      Modifier.fillMaxWidth().graphicsLayer {
                        translationX = -state.viewportState.scrollOffset.x * density.density
                      }
                  ) {
                    body(placedBundle)
                  }
                }
              },
            ) { measurables, viewportConstraints ->
              val contentConstraints =
                resolveEditorViewportContentConstraints(
                  viewportWidthPx = viewportConstraints.maxWidth,
                  contentWidthPx = resolvedContentWidth.dp.roundToPx(),
                )
              val placeable = measurables.single().measure(contentConstraints)
              val viewportSizeChanged =
                state.viewportState.updateMeasuredBounds(
                  viewportSize = measuredViewportSize,
                  contentSize = resolveSize(placeable.width, placeable.height),
                )
              if (viewportSizeChanged) {
                onMeasuredViewportSizeChange(measuredViewportSize)
              }

              val acceptedPreparation = currentPreparation.takeIf { acceptedBundle != null }

              val presentationScrollFrame =
                measuredScrollFrame.copy(state = placedBundle?.snapshot ?: scrollFrame.state)
              val scrollFrameVersion = presentationScrollFrame.state.version
              val bringIntoViewRequest = acceptedRequest?.takeIf {
                bringIntoViewRequests.activateForVersion(scrollFrameVersion) === it
              }
              var placementScrollY = state.viewportState.scrollOffset.y
              val scrollIntentResult = bringIntoViewRequest?.let { request ->
                acceptedPreparation?.scrollIntent
                  ?: resolveEditorScrollIntent(
                    frame = presentationScrollFrame,
                    target = request.target,
                    currentScroll = placementScrollY,
                  )
              }
              if (bringIntoViewRequest != null) {
                if (
                  state.viewportState.isTransforming ||
                    state.viewportState.isDirectManipulationInProgress
                ) {
                  bringIntoViewRequests.cancel()
                } else {
                  when (scrollIntentResult) {
                    null -> Unit
                    EditorScrollIntentResult.Unresolved -> Unit
                    EditorScrollIntentResult.NoScroll ->
                      requestToMarkPresented = bringIntoViewRequest
                    is EditorScrollIntentResult.ScrollTo -> {
                      smoothScrollJob?.cancel()
                      smoothScrollJob =
                        when (bringIntoViewRequest.behavior) {
                          EditorBringIntoViewBehavior.Instant -> {
                            val maximumScrollY =
                              acceptedPreparation?.maximumScrollY ?: state.viewportState.maxScrollY
                            placementScrollY = scrollIntentResult.y.coerceIn(0f, maximumScrollY)
                            state.viewportState.scrollToY(
                              targetY = placementScrollY,
                              isAutoScroll = true,
                              maximumScrollY = maximumScrollY,
                            )
                            requestToMarkPresented = bringIntoViewRequest
                            null
                          }

                          EditorBringIntoViewBehavior.Smooth ->
                            if (
                              bringIntoViewRequests.markPresented(
                                version = scrollFrameVersion,
                                request = bringIntoViewRequest,
                              )
                            ) {
                              coroutineScope.launch {
                                try {
                                  state.viewportState.animateScrollToY(
                                    targetY = scrollIntentResult.y,
                                    isAutoScroll = true,
                                  )
                                } finally {
                                  if (smoothScrollJob == coroutineContext[Job]) {
                                    smoothScrollJob = null
                                  }
                                }
                              }
                            } else {
                              null
                            }
                        }
                    }
                  }
                }
              } else {
                val reconciled =
                  scrollReconcileState.reconcile(
                    mode = viewportScrollReconcileMode,
                    viewportState = state.viewportState,
                    scrollFrame = presentationScrollFrame,
                    visibleArea = measuredVisibleArea,
                  )
                if (reconciled) placementScrollY = state.viewportState.scrollOffset.y
              }

              layout(width = viewportConstraints.maxWidth, height = viewportConstraints.maxHeight) {
                val scrollY = (placementScrollY * density.density).roundToInt()
                placeable.place(x = 0, y = -scrollY)
              }
            }
          }
          .map { it.measure(viewportConstraints) }
      val viewportSurfaceOverlayPlaceables =
        subcompose(EditorScreenLayoutSlot.ViewportSurfaceOverlay) {
            Box(modifier = Modifier.fillMaxSize().clipToBounds(), content = viewportSurfaceOverlay)
          }
          .map { it.measure(viewportConstraints) }

      layout(width = constraints.maxWidth, height = constraints.maxHeight) {
        viewportContentPlaceables.forEach { it.place(x = 0, y = 0) }
        viewportSurfaceOverlayPlaceables.forEach { it.place(x = 0, y = 0) }
        requestToMarkPresented?.let { request ->
          bringIntoViewRequests.markPresented(
            version = placedBundle?.snapshot?.version ?: scrollFrame.state.version,
            request = request,
          )
        }
        acceptedBundle?.let { bundle -> editor?.completePresentation(bundle) }
      }
    }

    NavigationForeground(sharePointerInputWithSiblings = true) {
      EditorViewportOverlayLayout(viewportOverlay)
    }
    NavigationForeground {
      EditorScreenForegroundLayout(
        overlay = overlay,
        toolbar = toolbar,
        subPane = subPane,
        softwareMagnifierSource = softwareMagnifierSource,
        magnifierPlacement = magnifierPlacement,
      )
    }
  }
}

private fun resolveEditorSurfacePreparation(
  editor: Editor,
  scrollFrame: EditorScrollFrame,
  currentScroll: Float,
  bringIntoViewRequest: EditorBringIntoViewRequests.Request?,
): EditorSurfacePreparation? {
  val state = scrollFrame.state
  val viewportHeight = scrollFrame.visibleArea.viewport.height
  val bodyGeometry =
    resolveEditorBodyGeometry(
      visibleArea = scrollFrame.visibleArea,
      layoutSpec = scrollFrame.layoutSpec,
      pageSizes = state.pageSizes,
      displayZoom = scrollFrame.displayZoom,
    )
  val editorTopInContainer = bodyGeometry.topSpacerHeight
  val headerHeight = scrollFrame.headerHeight.takeIf(Float::isFinite) ?: 0f
  val resolvedContentOrigin = headerHeight + editorTopInContainer
  val pageSpans =
    state.pageSizes.mapIndexedNotNull { page, size ->
      val pageTop =
        scrollFrame.layoutSpec.resolvePageContentTop(
          page = page,
          pageSizes = state.pageSizes,
          displayZoom = scrollFrame.displayZoom,
          density = scrollFrame.density,
        ) ?: return@mapIndexedNotNull null
      val top = resolvedContentOrigin + pageTop
      SurfacePageSpan(
        page = page,
        top = top,
        bottom =
          top +
            resolveMeasuredPageLength(
              length = size.height,
              displayZoom = scrollFrame.displayZoom,
              density = scrollFrame.density,
            ),
      )
    }
  val pagesContentHeight =
    scrollFrame.layoutSpec.resolvePagesContentHeight(
      pageSizes = state.pageSizes,
      displayZoom = scrollFrame.displayZoom,
      density = scrollFrame.density,
    )
  val contentExtent =
    headerHeight +
      maxOf(
        bodyGeometry.minimumBodyHeight,
        bodyGeometry.topSpacerHeight +
          pagesContentHeight +
          scrollFrame.autoScrollPolicy.bottomPadding,
      )
  val currentViewport =
    if (
      currentScroll.isFinite() &&
        currentScroll >= 0f &&
        viewportHeight.isFinite() &&
        viewportHeight > 0f
    ) {
      VerticalSpan(top = currentScroll, bottom = currentScroll + viewportHeight)
    } else {
      null
    }
  if (currentViewport == null) return null
  val scrollIntent = bringIntoViewRequest?.let { request ->
    resolveEditorScrollIntent(
      frame = scrollFrame,
      target = request.target,
      currentScroll = currentScroll,
      contentOriginY = resolvedContentOrigin,
    )
  }
  val maximumScrollY = (contentExtent - viewportHeight).coerceAtLeast(0f)
  val preparationViewports =
    if (bringIntoViewRequest?.behavior == EditorBringIntoViewBehavior.Instant) {
      resolveInstantRevealPreparationViewports(
        frame = scrollFrame,
        target = bringIntoViewRequest.target,
        currentScroll = currentScroll,
        contentOriginY = resolvedContentOrigin,
        maximumScrollY = maximumScrollY,
      )
    } else {
      emptyList()
    }
  val instantRevealUnresolved =
    bringIntoViewRequest?.behavior == EditorBringIntoViewBehavior.Instant &&
      scrollIntent == EditorScrollIntentResult.Unresolved
  if (instantRevealUnresolved) return null
  val requiredPages =
    requiredSurfacePages(
      pages = pageSpans,
      currentViewport = currentViewport,
      activePages = editor.activeSurfacePages,
      preparationViewports = preparationViewports,
    )
  val exactViewport =
    when {
      bringIntoViewRequest?.behavior != EditorBringIntoViewBehavior.Instant -> currentViewport
      scrollIntent is EditorScrollIntentResult.ScrollTo -> {
        val top = scrollIntent.y.coerceIn(0f, maximumScrollY)
        VerticalSpan(top = top, bottom = top + viewportHeight)
      }
      scrollIntent == EditorScrollIntentResult.NoScroll -> currentViewport
      else -> null
    }
  val exactPages = exactViewport?.let { viewport ->
    requiredSurfacePages(
      pages = pageSpans,
      currentViewport = viewport,
      activePages = editor.activeSurfacePages,
    )
  }
  if (bringIntoViewRequest?.behavior == EditorBringIntoViewBehavior.Instant) {
    check(exactPages != null && requiredPages.containsAll(exactPages)) {
      "Instant reveal destination requires unprepared surfaces: " +
        "required=$requiredPages destination=$exactPages"
    }
  }
  return EditorSurfacePreparation(
    requiredPages = requiredPages,
    scrollIntent = scrollIntent,
    maximumScrollY = maximumScrollY,
  )
}

@Composable
private fun EditorViewportOverlayLayout(viewportOverlay: @Composable BoxScope.() -> Unit) {
  SubcomposeLayout(modifier = Modifier.fillMaxSize().sharePointerInputWithSiblings()) { constraints
    ->
    val viewportConstraints =
      constraints.copy(
        minWidth = constraints.maxWidth,
        maxWidth = constraints.maxWidth,
        minHeight = constraints.maxHeight,
        maxHeight = constraints.maxHeight,
      )
    val viewportOverlayPlaceables =
      subcompose(EditorScreenLayoutSlot.ViewportOverlay) {
          Box(modifier = Modifier.fillMaxSize().clipToBounds(), content = viewportOverlay)
        }
        .map { it.measure(viewportConstraints) }

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      viewportOverlayPlaceables.forEach { it.place(x = 0, y = 0) }
    }
  }
}

@Composable
private fun EditorScreenForegroundLayout(
  overlay: @Composable () -> Unit,
  toolbar: @Composable () -> Unit,
  subPane: @Composable BoxScope.() -> Unit,
  softwareMagnifierSource: GraphicsLayer,
  magnifierPlacement: EditorMagnifierPlacement?,
) {
  SubcomposeLayout(
    modifier =
      Modifier.fillMaxSize()
        .editorSoftwareMagnifierLens(
          sourceLayer = softwareMagnifierSource,
          placement = magnifierPlacement,
        )
  ) { constraints ->
    val fullConstraints =
      constraints.copy(
        minWidth = constraints.maxWidth,
        maxWidth = constraints.maxWidth,
        minHeight = constraints.maxHeight,
        maxHeight = constraints.maxHeight,
      )
    val overlayPlaceables =
      subcompose(EditorScreenLayoutSlot.Overlay, overlay).map { it.measure(fullConstraints) }
    val toolbarPlaceables =
      subcompose(EditorScreenLayoutSlot.Toolbar, toolbar).map {
        it.measure(constraints.copy(minWidth = 0, minHeight = 0))
      }
    val subPanePlaceables =
      subcompose(EditorScreenLayoutSlot.SubPane) {
          Box(modifier = Modifier.fillMaxSize(), content = subPane)
        }
        .map { it.measure(fullConstraints) }

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      overlayPlaceables.forEach { it.place(x = 0, y = 0) }
      subPanePlaceables.forEach { it.place(x = 0, y = 0) }
      toolbarPlaceables.forEach { it.place(x = 0, y = constraints.maxHeight - it.height) }
    }
  }
}

internal class EditorViewportScrollReconcileState {
  private var lastObservedFrame: EditorViewportScrollReconcileFrame? = null

  fun reset() {
    lastObservedFrame = null
  }

  fun reconcile(
    mode: EditorViewportScrollReconcileMode,
    viewportState: EditorViewportState,
    scrollFrame: EditorScrollFrame,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    if (mode == EditorViewportScrollReconcileMode.Disabled) {
      reset()
      return false
    }
    if (viewportState.isTransforming || viewportState.isDirectManipulationInProgress) {
      return false
    }

    val frame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
    val previousFrame = lastObservedFrame
    if (previousFrame == null) {
      lastObservedFrame = frame
      return false
    }
    if (previousFrame.hasSameVisibleViewport(frame)) {
      lastObservedFrame = frame
      return false
    }

    return when (mode) {
      EditorViewportScrollReconcileMode.Disabled -> false
      EditorViewportScrollReconcileMode.KeepVisibleAnchor ->
        reconcileKeepVisibleAnchor(
          previousFrame = previousFrame,
          frame = frame,
          viewportState = viewportState,
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
        )
      EditorViewportScrollReconcileMode.RevealSelectionHead ->
        reconcileSelectionHeadReveal(
          scrollFrame = scrollFrame,
          viewportState = viewportState,
          visibleArea = visibleArea,
        )
    }
  }

  private fun reconcileKeepVisibleAnchor(
    previousFrame: EditorViewportScrollReconcileFrame,
    frame: EditorViewportScrollReconcileFrame,
    viewportState: EditorViewportState,
    scrollFrame: EditorScrollFrame,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    // visible area가 바뀌기 전 selection head가 이미 보이던 상태라면, 화면 중앙을 보존하지
    // 않고 selection head가 새 keep-visible 범위 안에 남도록만 보정한다.
    val wasSelectionVisible =
      isEditorScrollTargetVisible(
        frame = scrollFrame,
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        currentScroll = previousFrame.scrollY,
        visibleArea = previousFrame.visibleArea,
      )
    if (wasSelectionVisible == true) {
      return reconcileSelectionHeadReveal(
        scrollFrame = scrollFrame,
        viewportState = viewportState,
        visibleArea = visibleArea,
      )
    }

    val anchor = previousFrame.anchor
    val targetY = previousFrame.documentY(anchor) - frame.viewportY(anchor)
    if (abs(targetY - viewportState.scrollOffset.y) <= EditorViewportScrollAnchorTolerance) {
      lastObservedFrame = frame
      return false
    }

    viewportState.scrollToY(targetY = targetY, isAutoScroll = true)
    lastObservedFrame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
    return true
  }

  private fun reconcileSelectionHeadReveal(
    scrollFrame: EditorScrollFrame,
    viewportState: EditorViewportState,
    visibleArea: EditorVisibleArea,
  ): Boolean {
    return when (
      val scrollIntentResult =
        resolveEditorScrollIntent(
          frame = scrollFrame,
          target = EditorBringIntoViewTarget.CurrentSelectionHead,
          currentScroll = viewportState.scrollOffset.y,
        )
    ) {
      EditorScrollIntentResult.Unresolved,
      EditorScrollIntentResult.NoScroll -> {
        lastObservedFrame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
        false
      }
      is EditorScrollIntentResult.ScrollTo -> {
        viewportState.scrollToY(targetY = scrollIntentResult.y, isAutoScroll = true)
        lastObservedFrame = EditorViewportScrollReconcileFrame(viewportState, visibleArea)
        true
      }
    }
  }
}

private enum class EditorViewportScrollAnchor {
  Top,
  Center,
  Bottom,
}

private data class EditorViewportScrollReconcileFrame(
  val scrollY: Float,
  val maxScrollY: Float,
  val visibleArea: EditorVisibleArea,
  val visibleViewportTop: Float,
  val visibleViewportBottom: Float,
) {
  constructor(
    viewportState: EditorViewportState,
    visibleArea: EditorVisibleArea,
  ) : this(
    scrollY = viewportState.scrollOffset.y,
    maxScrollY = viewportState.maxScrollY,
    visibleArea = visibleArea,
    visibleViewportTop = visibleArea.visibleViewportTop,
    visibleViewportBottom = visibleArea.visibleViewportBottom,
  )

  val anchor: EditorViewportScrollAnchor
    get() =
      when {
        scrollY <= EditorViewportScrollAnchorTolerance -> EditorViewportScrollAnchor.Top
        maxScrollY - scrollY <= EditorViewportScrollAnchorTolerance ->
          EditorViewportScrollAnchor.Bottom
        else -> EditorViewportScrollAnchor.Center
      }

  fun hasSameVisibleViewport(other: EditorViewportScrollReconcileFrame): Boolean =
    visibleViewportTop == other.visibleViewportTop &&
      visibleViewportBottom == other.visibleViewportBottom

  fun documentY(anchor: EditorViewportScrollAnchor): Float = scrollY + viewportY(anchor)

  fun viewportY(anchor: EditorViewportScrollAnchor): Float =
    when (anchor) {
      EditorViewportScrollAnchor.Top -> visibleViewportTop
      EditorViewportScrollAnchor.Center -> (visibleViewportTop + visibleViewportBottom) / 2f
      EditorViewportScrollAnchor.Bottom -> visibleViewportBottom
    }
}

internal fun resolveEditorViewportContentWidth(
  viewportWidth: Float,
  contentTrackWidth: Float,
): Float = max(viewportWidth, contentTrackWidth).coerceAtLeast(0f)

internal fun resolveEditorViewportContentConstraints(
  viewportWidthPx: Int,
  contentWidthPx: Int,
): Constraints {
  val resolvedWidth = max(viewportWidthPx, contentWidthPx).coerceAtLeast(0)
  return Constraints(
    minWidth = resolvedWidth,
    maxWidth = resolvedWidth,
    minHeight = 0,
    maxHeight = Constraints.Infinity,
  )
}
