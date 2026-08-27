package co.typie.screen.editor.editor.layout

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PublishedBundle
import co.typie.editor.VerticalSpan
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.CapturedViewportAnchor
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.ResolvedViewportAnchor
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.ffi.ViewportAnchorResolution
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportAnchorGeometry
import co.typie.editor.viewport.EditorViewportAnchorState
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.resolveViewportAnchorContentOriginY
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest

class EditorViewportAnchorReconcilerTest {
  private val selectionAnchor = ViewportAnchor.Node(node = "1:1", offsetX = 0f, offsetY = 0f)
  private val movedSelectionAnchor = ViewportAnchor.Node(node = "1:2", offsetX = 0f, offsetY = 0f)
  private val viewportAnchor = ViewportAnchor.Node(node = "2:1", offsetX = 0f, offsetY = 0f)

  @Test
  fun `direct scroll restores preferred selection after it returns inside cursor guard`() =
    runTest {
      val visibleArea = visibleArea()
      val frame = frame(visibleArea)
      val anchorState = EditorViewportAnchorState()
      val viewportState = viewportState(scrollY = 100f)
      val editor = editor()

      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(selectionAnchor, anchorState.identity)

      viewportState.scrollToY(350f)
      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(viewportAnchor, anchorState.identity)

      viewportState.scrollToY(120f)
      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(selectionAnchor, anchorState.identity)
    }

  @Test
  fun `smooth reveal tracks the viewport center without replacing the running scroll`() = runTest {
    val visibleArea = visibleArea()
    val frame = frame(visibleArea)
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor()

    reconcile(editor, anchorState, frame, viewportState, visibleArea)
    viewportState.scrollToY(targetY = 250f, isAutoScroll = true)
    reconcile(
      editor = editor,
      anchorState = anchorState,
      frame = frame,
      viewportState = viewportState,
      visibleArea = visibleArea,
      smoothRevealActive = true,
    )

    assertEquals(viewportAnchor, anchorState.identity)
    assertEquals(Offset(x = 0f, y = 250f), viewportState.scrollOffset)
  }

  @Test
  fun `non-selection reveal keeps the viewport anchor when selection is outside the guard`() =
    runTest {
      val visibleArea = visibleArea()
      val frame = frame(visibleArea)
      val anchorState = EditorViewportAnchorState()
      val viewportState = viewportState(scrollY = 350f)
      val editor = editor(selectionY = 50f)

      reconcile(editor, anchorState, frame, viewportState, visibleArea)
      assertEquals(viewportAnchor, anchorState.identity)

      reconcileViewportAnchorObservation(
        editor = editor,
        anchorState = anchorState,
        bundle = PublishedBundle(snapshot = frame.state, frames = emptyMap()),
        frame = frame,
        viewportState = viewportState,
        visibleArea = visibleArea,
        mode = EditorViewportScrollReconcileMode.Enabled,
        smoothRevealActive = false,
        handoffTarget = EditorBringIntoViewTarget.TrackedItem("comment-1"),
        selectionRevealOrigin = null,
        contentOriginY = 0f,
      )

      assertEquals(viewportAnchor, anchorState.identity)
    }

  @Test
  fun `non-selection reveal handoff keeps the achieved scroll through the next publication`() =
    runTest {
      val visibleArea = visibleArea()
      val initialFrame =
        frame(visibleArea).let { frame ->
          frame.copy(
            state =
              frame.state.copy(
                selection =
                  Selection(
                    anchor = Position("text", 0, Affinity.Downstream),
                    head = Position("text", 0, Affinity.Downstream),
                  )
              )
          )
        }
      val candidateFrame = initialFrame.copy(state = initialFrame.state.copy(version = 2L))
      val anchorState =
        EditorViewportAnchorState().apply {
          attachSelection(
            selectionAnchor,
            anchorGeometry(240f),
            scrollOffset = Offset(x = 0f, y = 100f),
          )
        }
      val viewportState = viewportState(scrollY = 350f)
      val editor = editor(selectionY = 200f)

      attachViewportCenterAnchor(
        editor = editor,
        anchorState = anchorState,
        revision = initialFrame.state.version,
        frame = initialFrame,
        scrollOffset = viewportState.scrollOffset,
        contentOriginY = 0f,
      )
      anchorState.consumeScrollChange(viewportState.lastScrollRevision)
      anchorState.consumeVisibleAreaChange(visibleArea)

      viewportState.scrollToY(targetY = 349.75f, isAutoScroll = true)
      reconcileViewportAnchorObservation(
        editor = editor,
        anchorState = anchorState,
        bundle = PublishedBundle(snapshot = initialFrame.state, frames = emptyMap()),
        frame = initialFrame,
        viewportState = viewportState,
        visibleArea = visibleArea,
        mode = EditorViewportScrollReconcileMode.Enabled,
        smoothRevealActive = false,
        handoffTarget = EditorBringIntoViewTarget.TrackedItem("search-match:1"),
        selectionRevealOrigin = null,
        contentOriginY = 0f,
      )

      val publication =
        reconcileViewportAnchorPublication(
          editor = editor,
          anchorState = anchorState,
          publishedBundle = PublishedBundle(snapshot = initialFrame.state, frames = emptyMap()),
          candidateState = candidateFrame.state,
          measuredScrollFrame = candidateFrame,
          currentScrollOffset = viewportState.scrollOffset,
          maximumScrollY = viewportState.maxScrollY,
          contentOriginY = 0f,
        )
          as EditorViewportAnchorPublication.Ready

      assertEquals(349.75f, publication.scrollOffset.y)
    }

  @Test
  fun `transform-owned scroll does not reactivate a visible preferred selection`() = runTest {
    val visibleArea = visibleArea()
    val frame = frame(visibleArea)
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor()

    reconcile(editor, anchorState, frame, viewportState, visibleArea)
    assertEquals(selectionAnchor, anchorState.identity)

    viewportState.beginTransform()
    viewportState.scrollToTransformTarget(
      offset = Offset(x = 0f, y = 120f),
      retainUntilMeasuredBounds = false,
    )
    attachViewportZoomAnchor(
      editor = editor,
      anchorState = anchorState,
      revision = frame.state.version,
      anchor = EditorViewportAnchor(page = 0, x = 0f, y = 500f),
      displayPosition = Offset(x = 0f, y = 500f),
      scrollOffset = viewportState.scrollOffset,
      contentOriginY = 0f,
    )
    assertEquals(viewportAnchor, anchorState.identity)
    reconcile(editor, anchorState, frame, viewportState, visibleArea)
    viewportState.endTransform()
    reconcile(editor, anchorState, frame, viewportState, visibleArea)

    assertEquals(viewportAnchor, anchorState.identity)
  }

  @Test
  fun `repeated zoom attachments reuse the captured identity until their stable input changes`() =
    runTest {
      val anchorState = EditorViewportAnchorState()
      var firstEditorCaptures = 0
      val firstEditor =
        Editor(
          FakeFfiEditor(
            captureViewportAnchorAtProvider = { _, _ ->
              firstEditorCaptures += 1
              CapturedViewportAnchor(identity = viewportAnchor, geometry = selectionGeometry(500f))
            }
          ),
          this,
          StandardTestDispatcher(testScheduler),
        )
      var secondEditorCaptures = 0
      val secondEditor =
        Editor(
          FakeFfiEditor(
            captureViewportAnchorAtProvider = { _, _ ->
              secondEditorCaptures += 1
              CapturedViewportAnchor(identity = viewportAnchor, geometry = selectionGeometry(500f))
            }
          ),
          this,
          StandardTestDispatcher(testScheduler),
        )
      val anchor = EditorViewportAnchor(page = 0, x = 10f, y = 20f)

      fun attach(
        editor: Editor = firstEditor,
        revision: Long = 1L,
        point: EditorViewportAnchor = anchor,
        displayPosition: Offset = Offset(x = 100f, y = 200f),
        scrollOffset: Offset = Offset.Zero,
      ) {
        attachViewportZoomAnchor(
          editor = editor,
          anchorState = anchorState,
          revision = revision,
          anchor = point,
          displayPosition = displayPosition,
          scrollOffset = scrollOffset,
          contentOriginY = 0f,
        )
      }

      attach()
      attach(displayPosition = Offset(x = 120f, y = 230f), scrollOffset = Offset(x = 10f, y = 20f))
      assertEquals(1, firstEditorCaptures)
      assertEquals(110f, anchorState.pointAttachmentX)
      assertEquals(210f, anchorState.pointAttachmentY)

      attach(revision = 2L)
      attach(revision = 2L, point = anchor.copy(x = 11f))
      attach(editor = secondEditor, revision = 2L, point = anchor.copy(x = 11f))

      assertEquals(3, firstEditorCaptures)
      assertEquals(1, secondEditorCaptures)
    }

  @Test
  fun `publication proceeds when a live anchor has no candidate geometry`() = runTest {
    val visibleArea = visibleArea()
    val candidateFrame = frame(visibleArea).let { it.copy(state = it.state.copy(version = 2L)) }
    val anchorState =
      EditorViewportAnchorState().apply {
        attach(
          identity = selectionAnchor,
          geometry = co.typie.editor.viewport.EditorViewportAnchorGeometry(pointY = 200f),
          scrollOffset = Offset(x = 0f, y = 100f),
        )
      }
    val captured =
      CapturedViewportAnchor(
        identity = viewportAnchor,
        geometry =
          ResolvedViewportAnchor(
            point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 250f),
            rect = null,
          ),
      )
    val editor =
      Editor(
        FakeFfiEditor(
          captureViewportAnchorAtProvider = { _, _ -> captured },
          resolveViewportAnchorProvider = { _, _ -> ViewportAnchorResolution.NotLaidOut },
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )

    val publication =
      reconcileViewportAnchorPublication(
        editor = editor,
        anchorState = anchorState,
        publishedBundle =
          PublishedBundle(snapshot = candidateFrame.state.copy(version = 1L), frames = emptyMap()),
        candidateState = candidateFrame.state,
        measuredScrollFrame = candidateFrame,
        currentScrollOffset = Offset(x = 0f, y = 100f),
        maximumScrollY = 600f,
        contentOriginY = 0f,
      )

    assertEquals(
      EditorViewportAnchorPublication.Ready(
        scrollOffset = Offset(x = 0f, y = 100f),
        geometry = null,
      ),
      publication,
    )
  }

  @Test
  fun `observation keeps using the accepted presentation content origin`() = runTest {
    val visibleArea = visibleArea()
    val frame = frame(visibleArea)
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor()

    reconcileViewportAnchorObservation(
      editor = editor,
      anchorState = anchorState,
      bundle = PublishedBundle(snapshot = frame.state, frames = emptyMap()),
      frame = frame,
      viewportState = viewportState,
      visibleArea = visibleArea,
      mode = EditorViewportScrollReconcileMode.Enabled,
      smoothRevealActive = false,
      handoffTarget = null,
      selectionRevealOrigin = null,
      contentOriginY = 40f,
    )
    viewportState.scrollToY(targetY = 120f, isAutoScroll = true)
    reconcileViewportAnchorObservation(
      editor = editor,
      anchorState = anchorState,
      bundle = PublishedBundle(snapshot = frame.state, frames = emptyMap()),
      frame = frame,
      viewportState = viewportState,
      visibleArea = visibleArea,
      mode = EditorViewportScrollReconcileMode.Enabled,
      smoothRevealActive = false,
      handoffTarget = null,
      selectionRevealOrigin = null,
      contentOriginY = 40f,
    )

    assertEquals(120f, anchorState.pointAttachmentY)
  }

  @Test
  fun `visible area shrink moves the active rect only enough to keep it guarded`() = runTest {
    val visibleArea = visibleArea()
    val occluded = visibleArea(bottomOcclusionInset = 100f)
    val frame = frame(visibleArea)
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor(selectionY = 260f)

    reconcile(editor, anchorState, frame, viewportState, visibleArea)
    reconcile(editor, anchorState, frame.copy(visibleArea = occluded), viewportState, occluded)

    assertEquals(Offset(x = 0f, y = 130f), viewportState.scrollOffset)
    assertTrue(viewportState.lastScrollWasAuto)
  }

  @Test
  fun `selection change attaches without scrolling then viewport shrink keeps it guarded`() =
    runTest {
      val visibleArea = visibleArea()
      val occluded = visibleArea(bottomOcclusionInset = 100f)
      val initialFrame =
        frame(visibleArea).let { frame ->
          frame.copy(
            state =
              frame.state.copy(
                selection =
                  Selection(
                    anchor = Position("text", 0, Affinity.Downstream),
                    head = Position("text", 0, Affinity.Downstream),
                  )
              )
          )
        }
      val movedFrame =
        initialFrame.copy(
          state =
            initialFrame.state.copy(
              version = 2L,
              selection =
                Selection(
                  anchor = Position("text", 1, Affinity.Downstream),
                  head = Position("text", 1, Affinity.Downstream),
                ),
            )
        )
      val anchorState = EditorViewportAnchorState()
      val viewportState = viewportState(scrollY = 100f)
      var currentSelectionAnchor = selectionAnchor
      val geometries =
        mapOf(
          selectionAnchor to selectionGeometry(200f),
          movedSelectionAnchor to selectionGeometry(340f),
          viewportAnchor to
            ResolvedViewportAnchor(
              point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 500f),
              rect = null,
            ),
        )
      val editor =
        Editor(
          FakeFfiEditor(
            captureSelectionViewportAnchorProvider = {
              val geometry = geometries.getValue(currentSelectionAnchor)
              CapturedViewportAnchor(identity = currentSelectionAnchor, geometry = geometry)
            },
            captureViewportAnchorAtProvider = { _, _ ->
              CapturedViewportAnchor(
                identity = viewportAnchor,
                geometry = geometries.getValue(viewportAnchor),
              )
            },
            resolveViewportAnchorProvider = { _, anchor ->
              geometries[anchor]?.let(ViewportAnchorResolution::Resolved)
                ?: ViewportAnchorResolution.Deleted
            },
          ),
          this,
          StandardTestDispatcher(testScheduler),
        )

      reconcile(editor, anchorState, initialFrame, viewportState, visibleArea)
      currentSelectionAnchor = movedSelectionAnchor
      val publication =
        reconcileViewportAnchorPublication(
          editor = editor,
          anchorState = anchorState,
          publishedBundle = PublishedBundle(snapshot = initialFrame.state, frames = emptyMap()),
          candidateState = movedFrame.state,
          measuredScrollFrame = movedFrame,
          currentScrollOffset = viewportState.scrollOffset,
          maximumScrollY = viewportState.maxScrollY,
          contentOriginY = 0f,
        )

      assertEquals(movedSelectionAnchor, anchorState.identity)
      assertEquals(movedSelectionAnchor, anchorState.preferredSelectionIdentity)
      assertEquals(Offset(x = 0f, y = 100f), viewportState.scrollOffset)

      (publication as EditorViewportAnchorPublication.Ready).geometry?.let { geometry ->
        anchorState.acceptGeometry(geometry, viewportState.scrollOffset)
      }
      reconcile(editor, anchorState, movedFrame, viewportState, visibleArea)

      reconcile(
        editor,
        anchorState,
        movedFrame.copy(visibleArea = occluded),
        viewportState,
        occluded,
      )

      assertEquals(Offset(x = 0f, y = 210f), viewportState.scrollOffset)
      assertTrue(viewportState.lastScrollWasAuto)
    }

  @Test
  fun `initial selection outside the guard keeps the viewport center active`() = runTest {
    val visibleArea = visibleArea()
    val initialFrame =
      frame(visibleArea).let { frame ->
        frame.copy(
          state =
            frame.state.copy(
              selection =
                Selection(
                  anchor = Position("text", 0, Affinity.Downstream),
                  head = Position("text", 0, Affinity.Downstream),
                )
            )
        )
      }
    val anchorState = EditorViewportAnchorState()
    val viewportState = viewportState(scrollY = 100f)
    val editor = editor(selectionY = 500f)

    reconcileViewportAnchorPublication(
      editor = editor,
      anchorState = anchorState,
      publishedBundle = null,
      candidateState = initialFrame.state,
      measuredScrollFrame = initialFrame,
      currentScrollOffset = viewportState.scrollOffset,
      maximumScrollY = viewportState.maxScrollY,
      contentOriginY = 0f,
    )
    reconcile(editor, anchorState, initialFrame, viewportState, visibleArea)

    assertEquals(viewportAnchor, anchorState.identity)
    assertEquals(selectionAnchor, anchorState.preferredSelectionIdentity)
    assertEquals(Offset(x = 0f, y = 100f), viewportState.scrollOffset)
  }

  @Test
  fun `selection removal adopts the viewport center without scrolling`() = runTest {
    val visibleArea = visibleArea()
    val initialFrame =
      frame(visibleArea).let { frame ->
        frame.copy(
          state =
            frame.state.copy(
              selection =
                Selection(
                  anchor = Position("text", 0, Affinity.Downstream),
                  head = Position("text", 0, Affinity.Downstream),
                )
            )
        )
      }
    val candidateFrame =
      initialFrame.copy(state = initialFrame.state.copy(version = 2L, selection = null))
    val anchorState =
      EditorViewportAnchorState().apply {
        attachSelection(
          selectionAnchor,
          anchorGeometry(200f),
          scrollOffset = Offset(x = 0f, y = 100f),
        )
      }
    val editor =
      Editor(
        FakeFfiEditor(
          captureViewportAnchorAtProvider = { _, _ ->
            CapturedViewportAnchor(identity = viewportAnchor, geometry = selectionGeometry(500f))
          },
          resolveViewportAnchorProvider = { _, anchor ->
            when (anchor) {
              selectionAnchor -> ViewportAnchorResolution.Resolved(selectionGeometry(200f))
              viewportAnchor -> ViewportAnchorResolution.Resolved(selectionGeometry(500f))
              else -> ViewportAnchorResolution.Deleted
            }
          },
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )

    val publication =
      reconcileViewportAnchorPublication(
        editor = editor,
        anchorState = anchorState,
        publishedBundle = PublishedBundle(snapshot = initialFrame.state, frames = emptyMap()),
        candidateState = candidateFrame.state,
        measuredScrollFrame = candidateFrame,
        currentScrollOffset = Offset(x = 0f, y = 100f),
        maximumScrollY = 600f,
        contentOriginY = resolveViewportAnchorContentOriginY(candidateFrame),
      )

    assertEquals(viewportAnchor, anchorState.identity)
    assertEquals(null, anchorState.preferredSelectionIdentity)
    assertEquals(100f, (publication as EditorViewportAnchorPublication.Ready).scrollOffset.y)
  }

  @Test
  fun `unresolved candidate selection keeps the existing anchors and publishes`() = runTest {
    val visibleArea = visibleArea()
    val initialFrame =
      frame(visibleArea).let { frame ->
        frame.copy(
          state =
            frame.state.copy(
              selection =
                Selection(
                  anchor = Position("text", 0, Affinity.Downstream),
                  head = Position("text", 0, Affinity.Downstream),
                )
            )
        )
      }
    val candidateFrame = initialFrame.copy(state = initialFrame.state.copy(version = 2L))
    val anchorState =
      EditorViewportAnchorState().apply {
        attachSelection(
          selectionAnchor,
          anchorGeometry(200f),
          scrollOffset = Offset(x = 0f, y = 100f),
        )
      }
    val editor =
      Editor(
        FakeFfiEditor(
          captureSelectionViewportAnchorProvider = { null },
          resolveViewportAnchorProvider = { _, _ ->
            ViewportAnchorResolution.Resolved(selectionGeometry(200f))
          },
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )

    val publication =
      reconcileViewportAnchorPublication(
        editor = editor,
        anchorState = anchorState,
        publishedBundle = PublishedBundle(snapshot = initialFrame.state, frames = emptyMap()),
        candidateState = candidateFrame.state,
        measuredScrollFrame = candidateFrame,
        currentScrollOffset = Offset(x = 0f, y = 100f),
        maximumScrollY = 600f,
        contentOriginY = 0f,
      )

    assertTrue(publication is EditorViewportAnchorPublication.Ready)
    assertEquals(selectionAnchor, anchorState.identity)
    assertEquals(selectionAnchor, anchorState.preferredSelectionIdentity)
  }

  private fun reconcile(
    editor: Editor,
    anchorState: EditorViewportAnchorState,
    frame: EditorScrollFrame,
    viewportState: EditorViewportState,
    visibleArea: EditorVisibleArea,
    smoothRevealActive: Boolean = false,
  ) {
    reconcileViewportAnchorObservation(
      editor = editor,
      anchorState = anchorState,
      bundle = PublishedBundle(snapshot = frame.state, frames = emptyMap()),
      frame = frame,
      viewportState = viewportState,
      visibleArea = visibleArea,
      mode = EditorViewportScrollReconcileMode.Enabled,
      smoothRevealActive = smoothRevealActive,
      handoffTarget = null,
      selectionRevealOrigin = null,
      contentOriginY = 0f,
    )
  }

  private fun TestScope.editor(selectionY: Float = 200f): Editor {
    val selectionRect = PageRect(pageIdx = 0, rect = Rect(0f, selectionY - 10f, 1f, 20f))
    return Editor(
      FakeFfiEditor(
        captureSelectionViewportAnchorProvider = {
          CapturedViewportAnchor(
            identity = selectionAnchor,
            geometry =
              ResolvedViewportAnchor(
                point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = selectionY),
                rect = selectionRect,
              ),
          )
        },
        captureViewportAnchorAtProvider = { _, _ ->
          CapturedViewportAnchor(
            identity = viewportAnchor,
            geometry =
              ResolvedViewportAnchor(
                point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 500f),
                rect = null,
              ),
          )
        },
        resolveViewportAnchorProvider = { _, anchor ->
          val resolved =
            when (anchor) {
              selectionAnchor ->
                ResolvedViewportAnchor(
                  point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = selectionY),
                  rect = selectionRect,
                )
              viewportAnchor ->
                ResolvedViewportAnchor(
                  point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = 500f),
                  rect = null,
                )
              else -> null
            }
          if (resolved == null) {
            ViewportAnchorResolution.Deleted
          } else {
            ViewportAnchorResolution.Resolved(resolved)
          }
        },
      ),
      this,
      StandardTestDispatcher(testScheduler),
    )
  }

  private fun selectionGeometry(selectionY: Float): ResolvedViewportAnchor =
    ResolvedViewportAnchor(
      point = ViewportAnchorPoint(pageIdx = 0, x = 0f, y = selectionY),
      rect = PageRect(pageIdx = 0, rect = Rect(0f, selectionY - 10f, 1f, 20f)),
    )

  private fun anchorGeometry(selectionY: Float): EditorViewportAnchorGeometry =
    EditorViewportAnchorGeometry(
      pointY = selectionY,
      rect = VerticalSpan(top = selectionY - 10f, bottom = selectionY + 10f),
    )

  private fun viewportState(scrollY: Float): EditorViewportState =
    EditorViewportState().apply {
      updateMeasuredBounds(
        viewportSize = Size(width = 300f, height = 300f),
        contentSize = Size(width = 300f, height = 900f),
      )
      scrollToY(scrollY)
    }

  private fun visibleArea(bottomOcclusionInset: Float = 0f): EditorVisibleArea =
    EditorVisibleArea(
      viewport = Size(width = 300f, height = 300f),
      bottomOcclusionInset = bottomOcclusionInset,
    )

  private fun frame(visibleArea: EditorVisibleArea): EditorScrollFrame =
    EditorScrollFrame(
      state =
        EditorState(
          version = 1L,
          cursor = null,
          selection = null,
          pageSizes = listOf(PageSize(width = 300f, height = 900f)),
          externalElements = emptyList(),
          rootAttrs = null,
          rootModifiers = null,
          ime = null,
        ),
      layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 300f),
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea = visibleArea),
      headerHeight = 0f,
      density = 1f,
      editorBounds = EditorBoundsInContainer(x = 0f, y = 0f, width = 300f, height = 900f),
    )
}
