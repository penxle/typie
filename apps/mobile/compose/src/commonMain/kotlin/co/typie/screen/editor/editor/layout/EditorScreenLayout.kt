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
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.MotionDurationScale
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
import co.typie.editor.body.resolveMeasuredPageLength
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
import co.typie.editor.scroll.resolveEditorScrollIntent
import co.typie.editor.scroll.resolveInstantRevealPreparationViewports
import co.typie.editor.viewport.EditorViewportAnchorRevealOrigin
import co.typie.editor.viewport.EditorViewportAnchorState
import co.typie.editor.viewport.resolveViewportAnchorContentOriginY
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
import co.typie.ui.input.PointerInputModeState
import co.typie.ui.input.trackPointerInputMode
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

private enum class EditorScreenLayoutSlot {
  ViewportContent,
  ViewportSurfaceOverlay,
  ViewportOverlay,
  Overlay,
  Toolbar,
  SubPane,
}

private object EditorViewportNestedScrollConnection : NestedScrollConnection

internal data class EditorSurfacePreparation(
  val requiredPages: Set<Int>,
  val scrollIntent: EditorScrollIntentResult?,
  val maximumScrollY: Float,
  val contentOriginY: Float,
  val anchorPublication: EditorViewportAnchorPublication.Ready? = null,
)

private data class EditorViewportAnchorPresentation(
  val editor: Editor,
  val bundle: PublishedBundle,
  val frame: EditorScrollFrame,
  val visibleArea: EditorVisibleArea,
  val contentOriginY: Float,
)

private class EditorViewportAnchorPresentationRef {
  var current: EditorViewportAnchorPresentation? = null
}

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
  Enabled,
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
internal fun EditorScreenLayout(
  state: EditorScreenState,
  editor: Editor? = null,
  scrollFrame: EditorScrollFrame,
  visibleArea: EditorVisibleArea,
  magnifierFocalPositionInRoot: Offset? = null,
  viewportScrollableState: Scrollable2DState,
  viewportContentWidth: Float,
  viewportAnchorState: EditorViewportAnchorState? = null,
  isCurrentNavigationRoute: Boolean = true,
  editorInteractionEnabled: Boolean = true,
  platformIndirectScaleEnabled: Boolean = editorInteractionEnabled,
  viewportScrollReconcileMode: EditorViewportScrollReconcileMode,
  pointerInputModeState: PointerInputModeState? = null,
  onViewportIndirectInput: () -> Unit = {},
  onViewportPointerEnter: () -> Unit = {},
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
  val activePointerInputModeState = pointerInputModeState ?: remember { PointerInputModeState() }
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val platformIndirectScaleBridge = remember { EditorPlatformIndirectScaleBridge() }
  val viewConfiguration = LocalViewConfiguration.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val coroutineScope = rememberCoroutineScope()
  val smoothScrollSession =
    remember(coroutineScope, state.viewportState, bringIntoViewRequests) {
      EditorSmoothScrollSession(coroutineScope, state.viewportState, bringIntoViewRequests)
    }
  val smoothScrollEnabled =
    coroutineScope.coroutineContext[MotionDurationScale]
      ?.scaleFactor
      ?.takeIf { it.isFinite() }
      ?.let { it > 0f } ?: true
  val appliedState = editor?.appliedState
  val bringIntoViewRequest = appliedState?.let {
    bringIntoViewRequests.activateForVersion(it.version)
  }
  val ownedViewportAnchorState = remember { EditorViewportAnchorState() }
  val activeViewportAnchorState = viewportAnchorState ?: ownedViewportAnchorState
  val surfacePreparation = editor?.let {
    resolveAnchoredEditorSurfacePreparation(
      editor = it,
      scrollFrame = scrollFrame.withState(it.appliedState),
      currentScrollOffset = state.viewportState.scrollOffset,
      bringIntoViewRequest = bringIntoViewRequest,
      anchorState = activeViewportAnchorState,
      publishedBundle = it.publishedBundle,
      smoothScrollEnabled = smoothScrollEnabled,
      smoothRevealActive = smoothScrollSession.active,
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
  val viewportAnchorPresentationRef = remember { EditorViewportAnchorPresentationRef() }
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
      smoothScrollSession.stop()?.let(bringIntoViewRequests::discard)
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
        .trackPointerInputMode(
          state = activePointerInputModeState,
          onNonTouchPointerEnter = onViewportPointerEnter,
        )
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
          resolveAnchoredEditorSurfacePreparation(
            editor = editor,
            scrollFrame = measuredScrollFrame.withState(currentAppliedState),
            currentScrollOffset = state.viewportState.scrollOffset,
            bringIntoViewRequest = currentRequest,
            anchorState = activeViewportAnchorState,
            publishedBundle = publishedBundle,
            smoothScrollEnabled = smoothScrollEnabled,
            smoothRevealActive = smoothScrollSession.active,
          )
        } else {
          null
        }
      val candidateBundle = currentPreparation?.let {
        editor?.publishIfReady(requiredPages = it.requiredPages)
      }
      val acceptedBundle = candidateBundle?.let { candidate ->
        val acceptingEditor = editor ?: return@let null
        val latestState = acceptingEditor.appliedState
        val latestRequest = bringIntoViewRequests.activateForVersion(latestState.version)
        val stillCurrent =
          latestState === candidate.snapshot &&
            latestRequest === currentRequest &&
            currentPreparation ==
              resolveAnchoredEditorSurfacePreparation(
                editor = acceptingEditor,
                scrollFrame = measuredScrollFrame.withState(latestState),
                currentScrollOffset = state.viewportState.scrollOffset,
                bringIntoViewRequest = latestRequest,
                anchorState = activeViewportAnchorState,
                publishedBundle = publishedBundle,
                smoothScrollEnabled = smoothScrollEnabled,
                smoothRevealActive = smoothScrollSession.active,
              )
        if (!stillCurrent) return@let null

        val anchorPublication = currentPreparation.anchorPublication ?: return@let null
        if (!acceptingEditor.acceptPublication(candidate)) {
          return@let null
        }

        val previousScrollOffset = state.viewportState.scrollOffset
        state.viewportState.scrollTo(
          offset = anchorPublication.scrollOffset,
          isAutoScroll = true,
          maximumScrollOffset =
            Offset(
              x = resolveMaximumScrollX(measuredScrollFrame.withState(candidate.snapshot)),
              y = currentPreparation.maximumScrollY,
            ),
        )
        smoothScrollSession.translate(state.viewportState.scrollOffset.y - previousScrollOffset.y)
        anchorPublication.geometry?.let { geometry ->
          if (
            anchorPublication.attachmentAchieved &&
              (state.viewportState.scrollOffset - anchorPublication.scrollOffset).getDistance() <=
                1f
          ) {
            activeViewportAnchorState.acceptGeometry(geometry, state.viewportState.scrollOffset)
          }
        }
        candidate
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
                measuredScrollFrame.withState(placedBundle?.snapshot ?: scrollFrame.state)
              val presentationContentOriginY =
                acceptedPreparation?.contentOriginY
                  ?: resolveViewportAnchorContentOriginY(presentationScrollFrame)
              viewportAnchorPresentationRef.current =
                if (editor != null && placedBundle != null) {
                  EditorViewportAnchorPresentation(
                    editor = editor,
                    bundle = placedBundle,
                    frame = presentationScrollFrame,
                    visibleArea = measuredVisibleArea,
                    contentOriginY = presentationContentOriginY,
                  )
                } else {
                  null
                }
              val scrollFrameVersion = presentationScrollFrame.state.version
              val bringIntoViewRequest = acceptedRequest?.takeIf {
                bringIntoViewRequests.activateForVersion(scrollFrameVersion) === it
              }
              var placementScrollY = state.viewportState.scrollOffset.y
              var viewportAnchorHandoffTarget: EditorBringIntoViewTarget? = null
              var handoffSelectionRevealOrigin: EditorViewportAnchorRevealOrigin? = null
              val scrollIntentResult = bringIntoViewRequest?.let { request ->
                acceptedPreparation?.scrollIntent
                  ?: resolveEditorScrollIntent(
                    frame = presentationScrollFrame,
                    target = request.target,
                    policy = request.policy,
                    currentScroll = placementScrollY,
                    contentOriginY = presentationContentOriginY,
                    maximumScrollY = state.viewportState.maxScrollY,
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
                    EditorScrollIntentResult.NoScroll -> {
                      if (!smoothScrollSession.finishIfNearTarget(bringIntoViewRequest)) {
                        smoothScrollSession.stop()
                      }
                      placementScrollY = state.viewportState.scrollOffset.y
                      requestToMarkPresented = bringIntoViewRequest
                      viewportAnchorHandoffTarget = bringIntoViewRequest.target
                      if (
                        bringIntoViewRequest.target ==
                          EditorBringIntoViewTarget.CurrentSelectionHead
                      ) {
                        handoffSelectionRevealOrigin =
                          EditorViewportAnchorRevealOrigin(
                            scrollY = placementScrollY,
                            target = bringIntoViewRequest.target,
                            policy = bringIntoViewRequest.policy,
                          )
                      }
                    }
                    is EditorScrollIntentResult.ScrollTo -> {
                      val effectiveBehavior =
                        if (smoothScrollEnabled) {
                          bringIntoViewRequest.behavior
                        } else {
                          EditorBringIntoViewBehavior.Instant
                        }
                      when (effectiveBehavior) {
                        EditorBringIntoViewBehavior.Instant -> {
                          val revealOriginScrollY = placementScrollY
                          smoothScrollSession.stop()
                          val maximumScrollY =
                            acceptedPreparation?.maximumScrollY ?: state.viewportState.maxScrollY
                          placementScrollY = scrollIntentResult.y.coerceIn(0f, maximumScrollY)
                          state.viewportState.scrollToY(
                            targetY = placementScrollY,
                            isAutoScroll = true,
                            maximumScrollY = maximumScrollY,
                          )
                          requestToMarkPresented = bringIntoViewRequest
                          viewportAnchorHandoffTarget = bringIntoViewRequest.target
                          if (
                            bringIntoViewRequest.target ==
                              EditorBringIntoViewTarget.CurrentSelectionHead
                          ) {
                            handoffSelectionRevealOrigin =
                              EditorViewportAnchorRevealOrigin(
                                scrollY = revealOriginScrollY,
                                target = bringIntoViewRequest.target,
                                policy = bringIntoViewRequest.policy,
                              )
                          }
                        }

                        EditorBringIntoViewBehavior.Smooth -> {
                          val maximumScrollY =
                            acceptedPreparation?.maximumScrollY ?: state.viewportState.maxScrollY
                          val smoothScrollUpdate =
                            smoothScrollSession.retarget(
                              request = bringIntoViewRequest,
                              targetY = scrollIntentResult.y,
                              viewportHeight = state.viewportState.viewportSize.height,
                              maximumScrollY = maximumScrollY,
                            ) { completedRequest ->
                              val activePresentation = viewportAnchorPresentationRef.current
                              if (
                                activePresentation != null &&
                                  activePresentation.bundle.snapshot !==
                                    activePresentation.editor.appliedState
                              ) {
                                activePresentation.editor.requestPublication()
                                false
                              } else {
                                if (
                                  activePresentation != null &&
                                    bringIntoViewRequests.markPresented(
                                      version = activePresentation.bundle.snapshot.version,
                                      request = completedRequest,
                                    )
                                ) {
                                  attachSelectionViewportAnchor(
                                    editor = activePresentation.editor,
                                    anchorState = activeViewportAnchorState,
                                    revision = activePresentation.bundle.snapshot.version,
                                    frame = activePresentation.frame,
                                    scrollOffset = state.viewportState.scrollOffset,
                                    visibleArea = activePresentation.visibleArea,
                                    requireGuard =
                                      completedRequest.target !=
                                        EditorBringIntoViewTarget.CurrentSelectionHead,
                                    contentOriginY = activePresentation.contentOriginY,
                                  )
                                }
                                true
                              }
                            }
                          if (smoothScrollUpdate == EditorSmoothScrollUpdate.Changed) {
                            editor?.let { activeEditor ->
                              attachViewportCenterAnchor(
                                editor = activeEditor,
                                anchorState = activeViewportAnchorState,
                                revision = scrollFrameVersion,
                                frame = presentationScrollFrame,
                                scrollOffset = state.viewportState.scrollOffset,
                                contentOriginY = presentationContentOriginY,
                              )
                            }
                          }
                          if (smoothScrollUpdate == EditorSmoothScrollUpdate.Finished) {
                            requestToMarkPresented = bringIntoViewRequest
                            viewportAnchorHandoffTarget = bringIntoViewRequest.target
                          }
                        }
                      }
                    }
                  }
                }
              } else {
                placementScrollY = state.viewportState.scrollOffset.y
              }

              reconcileViewportAnchorObservation(
                editor = editor,
                anchorState = activeViewportAnchorState,
                bundle = placedBundle,
                frame = presentationScrollFrame,
                viewportState = state.viewportState,
                visibleArea = measuredVisibleArea,
                mode = viewportScrollReconcileMode,
                smoothRevealActive = smoothScrollSession.active,
                handoffTarget = viewportAnchorHandoffTarget,
                selectionRevealOrigin = handoffSelectionRevealOrigin,
                contentOriginY = presentationContentOriginY,
              )
              placementScrollY = state.viewportState.scrollOffset.y

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

internal fun resolveAnchoredEditorSurfacePreparation(
  editor: Editor,
  scrollFrame: EditorScrollFrame,
  currentScrollOffset: Offset,
  bringIntoViewRequest: EditorBringIntoViewRequests.Request?,
  anchorState: EditorViewportAnchorState,
  publishedBundle: PublishedBundle?,
  smoothScrollEnabled: Boolean = true,
  smoothRevealActive: Boolean = false,
): EditorSurfacePreparation? {
  val initial =
    resolveEditorSurfacePreparation(
      editor = editor,
      scrollFrame = scrollFrame,
      currentScroll = currentScrollOffset.y,
      bringIntoViewRequest = bringIntoViewRequest,
      smoothScrollEnabled = smoothScrollEnabled,
    ) ?: return null
  val anchorPublication =
    reconcileViewportAnchorPublication(
      editor = editor,
      anchorState = anchorState,
      publishedBundle = publishedBundle,
      candidateState = scrollFrame.state,
      measuredScrollFrame = scrollFrame,
      currentScrollOffset = currentScrollOffset,
      maximumScrollY = initial.maximumScrollY,
      maximumScrollX = resolveMaximumScrollX(scrollFrame),
      contentOriginY = initial.contentOriginY,
      smoothRevealActive = smoothRevealActive,
    )
      as? EditorViewportAnchorPublication.Ready ?: return null
  if (anchorPublication.scrollOffset == currentScrollOffset) {
    return initial.copy(anchorPublication = anchorPublication)
  }
  return resolveEditorSurfacePreparation(
      editor = editor,
      scrollFrame = scrollFrame,
      currentScroll = anchorPublication.scrollOffset.y,
      bringIntoViewRequest = bringIntoViewRequest,
      smoothScrollEnabled = smoothScrollEnabled,
    )
    ?.copy(anchorPublication = anchorPublication)
}

private fun resolveMaximumScrollX(frame: EditorScrollFrame): Float {
  return (frame.bodyGeometry.pageColumnWidth - frame.visibleArea.viewport.width).coerceAtLeast(0f)
}

internal fun resolveEditorSurfacePreparation(
  editor: Editor,
  scrollFrame: EditorScrollFrame,
  currentScroll: Float,
  bringIntoViewRequest: EditorBringIntoViewRequests.Request?,
  smoothScrollEnabled: Boolean = true,
): EditorSurfacePreparation? {
  val state = scrollFrame.state
  val viewportHeight = scrollFrame.visibleArea.viewport.height
  val bodyGeometry = scrollFrame.bodyGeometry
  val headerHeight = scrollFrame.headerHeight.takeIf(Float::isFinite) ?: 0f
  val resolvedContentOrigin = headerHeight + bodyGeometry.topSpacerHeight
  val pageSpans =
    state.pageSizes.mapIndexedNotNull { page, size ->
      val pageTop = scrollFrame.pageContentTop(page) ?: return@mapIndexedNotNull null
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
  val pagesContentHeight = scrollFrame.pagesContentHeight
  val contentExtent =
    headerHeight +
      maxOf(
        bodyGeometry.minimumBodyHeight,
        bodyGeometry.topSpacerHeight +
          pagesContentHeight +
          scrollFrame.autoScrollPolicy.bottomPadding,
      )
  if (!currentScroll.isFinite() || !viewportHeight.isFinite() || viewportHeight <= 0f) return null
  val maximumScrollY = (contentExtent - viewportHeight).coerceAtLeast(0f)
  val planningScrollY = currentScroll.coerceIn(0f, maximumScrollY)
  val currentViewport =
    VerticalSpan(top = planningScrollY, bottom = planningScrollY + viewportHeight)
  val scrollIntent = bringIntoViewRequest?.let { request ->
    resolveEditorScrollIntent(
      frame = scrollFrame,
      target = request.target,
      policy = request.policy,
      currentScroll = planningScrollY,
      contentOriginY = resolvedContentOrigin,
      maximumScrollY = maximumScrollY,
    )
  }
  val instantReveal =
    bringIntoViewRequest != null &&
      (bringIntoViewRequest.behavior == EditorBringIntoViewBehavior.Instant || !smoothScrollEnabled)
  val preparationViewports =
    if (instantReveal) {
      resolveInstantRevealPreparationViewports(
        frame = scrollFrame,
        target = bringIntoViewRequest.target,
        policy = bringIntoViewRequest.policy,
        currentScroll = planningScrollY,
        contentOriginY = resolvedContentOrigin,
        maximumScrollY = maximumScrollY,
      )
    } else {
      emptyList()
    }
  val instantRevealUnresolved = instantReveal && scrollIntent == EditorScrollIntentResult.Unresolved
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
      !instantReveal -> currentViewport
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
  if (instantReveal) {
    check(exactPages != null && requiredPages.containsAll(exactPages)) {
      "Instant reveal destination requires unprepared surfaces: " +
        "required=$requiredPages destination=$exactPages"
    }
  }
  return EditorSurfacePreparation(
    requiredPages = requiredPages,
    scrollIntent = scrollIntent,
    maximumScrollY = maximumScrollY,
    contentOriginY = resolvedContentOrigin,
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
