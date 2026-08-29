package co.typie.screen.editor.editor.layout

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.Editor
import co.typie.editor.EditorState
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
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.ffi.ViewportAnchorResolution
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorScrollIntentResult
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportAnchorGeometry
import co.typie.editor.viewport.EditorViewportAnchorRevealOrigin
import co.typie.editor.viewport.EditorViewportAnchorState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class EditorSurfacePreparationTest {
  @Test
  fun `measured selection reveal converges before editor bounds are mounted`() = runTest {
    val identity = ViewportAnchor.Node(node = "1:1", offsetX = 0f, offsetY = 0f)
    val anchor = Position(node = "1:1", offset = 0, affinity = Affinity.Downstream)
    val head = Position(node = "1:1", offset = 1, affinity = Affinity.Downstream)
    val provisionalRect = PageRect(pageIdx = 2, rect = Rect(0f, 100f, 1f, 1f))
    val measuredRect = PageRect(pageIdx = 2, rect = Rect(0f, 100f, 1f, 300f))
    val baseFrame = frame().copy(editorBounds = EditorBoundsInContainer())
    fun selectionFrame(version: Long, rect: PageRect): EditorScrollFrame =
      baseFrame.copy(
        state =
          baseFrame.state.copy(
            version = version,
            selection = Selection(anchor = anchor, head = head),
            selectionEndpoints =
              SelectionEndpoints(from = rect, to = rect, fromPosition = anchor, toPosition = head),
          )
      )

    val provisionalFrame = selectionFrame(version = 1L, rect = provisionalRect)
    val measuredFrame = selectionFrame(version = 2L, rect = measuredRect)
    val target = EditorBringIntoViewTarget.CurrentSelectionHead
    val policy = EditorBringIntoViewPolicy.CursorGuard
    val request =
      EditorBringIntoViewRequests.Request(
        target = target,
        policy = policy,
        behavior = EditorBringIntoViewBehavior.Instant,
      )
    val provisionalScroll =
      (resolveEditorSurfacePreparation(
            editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)),
            scrollFrame = provisionalFrame,
            currentScroll = 0f,
            bringIntoViewRequest = request,
          )
          ?.scrollIntent as EditorScrollIntentResult.ScrollTo)
        .y
    val editor =
      Editor(
        FakeFfiEditor(
          captureSelectionViewportAnchorProvider = {
            CapturedViewportAnchor(
              identity = identity,
              geometry =
                ResolvedViewportAnchor(
                  point = ViewportAnchorPoint(pageIdx = 2, x = 0f, y = 250f),
                  rect = measuredRect,
                ),
            )
          },
          resolveViewportAnchorProvider = { _, _ ->
            ViewportAnchorResolution.Resolved(
              ResolvedViewportAnchor(
                point = ViewportAnchorPoint(pageIdx = 2, x = 0f, y = 250f),
                rect = measuredRect,
              )
            )
          },
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    val anchorState =
      EditorViewportAnchorState().apply {
        attachSelection(
          identity = identity,
          geometry =
            EditorViewportAnchorGeometry(pointY = 2_100.5f, rect = VerticalSpan(2_100f, 2_101f)),
          scrollOffset = Offset(x = 0f, y = provisionalScroll),
          revealOrigin =
            EditorViewportAnchorRevealOrigin(scrollY = 0f, target = target, policy = policy),
        )
      }
    val expectedScroll =
      (resolveEditorSurfacePreparation(
            editor = editor,
            scrollFrame = measuredFrame,
            currentScroll = 0f,
            bringIntoViewRequest = request,
          )
          ?.scrollIntent as EditorScrollIntentResult.ScrollTo)
        .y

    val preparation =
      resolveAnchoredEditorSurfacePreparation(
        editor = editor,
        scrollFrame = measuredFrame,
        currentScrollOffset = Offset(x = 0f, y = provisionalScroll),
        bringIntoViewRequest = null,
        anchorState = anchorState,
        publishedBundle = PublishedBundle(snapshot = provisionalFrame.state, frames = emptyMap()),
      )

    assertEquals(expectedScroll, preparation?.anchorPublication?.scrollOffset?.y)
    assertEquals(
      preparation!!.contentOriginY + 2_100f,
      preparation.anchorPublication?.geometry?.rect?.top,
    )
  }

  @Test
  fun `surface planning and reveal use the anchor corrected candidate scroll`() = runTest {
    val identity = ViewportAnchor.Node(node = "1:1", offsetX = 0f, offsetY = 0f)
    val editor =
      Editor(
        FakeFfiEditor(
          resolveViewportAnchorProvider = { _, _ ->
            ViewportAnchorResolution.Resolved(
              ResolvedViewportAnchor(
                point = ViewportAnchorPoint(pageIdx = 2, x = 0f, y = 100f),
                rect = null,
              )
            )
          }
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    val target = EditorBringIntoViewTarget.TrackedItem("comment-1")
    val frame = frameWithTrackedItem(target.id)
    val request =
      EditorBringIntoViewRequests.Request(
        target = target,
        policy = EditorBringIntoViewPolicy.Reveal,
        behavior = EditorBringIntoViewBehavior.Instant,
      )

    val uncorrected =
      resolveEditorSurfacePreparation(
        editor = editor,
        scrollFrame = frame,
        currentScroll = 100f,
        bringIntoViewRequest = request,
      )
    val anchorState =
      EditorViewportAnchorState().apply {
        attach(
          identity = identity,
          geometry = EditorViewportAnchorGeometry(pointY = 200f, rect = VerticalSpan(190f, 210f)),
          scrollOffset = Offset(x = 0f, y = 100f),
        )
      }
    val corrected =
      resolveAnchoredEditorSurfacePreparation(
        editor = editor,
        scrollFrame = frame,
        currentScrollOffset = Offset(x = 0f, y = 100f),
        bringIntoViewRequest = request,
        anchorState = anchorState,
        publishedBundle =
          PublishedBundle(snapshot = frame.state.copy(version = 1L), frames = emptyMap()),
      )
    val measuredInitially =
      resolveEditorSurfacePreparation(
        editor = editor,
        scrollFrame = frame,
        currentScroll = 2_000f,
        bringIntoViewRequest = request,
      )

    assertNotEquals(uncorrected, corrected)
    assertEquals(measuredInitially?.requiredPages, corrected?.requiredPages)
    assertEquals(measuredInitially?.scrollIntent, corrected?.scrollIntent)
    assertEquals(measuredInitially?.maximumScrollY, corrected?.maximumScrollY)
  }

  @Test
  fun `disabled smooth motion prepares the destination like an instant reveal`() = runTest {
    val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
    val target = EditorBringIntoViewTarget.TrackedItem("comment-1")
    val frame = frameWithTrackedItem(target.id)
    val request =
      EditorBringIntoViewRequests.Request(
        target = target,
        policy = EditorBringIntoViewPolicy.Reveal,
        behavior = EditorBringIntoViewBehavior.Smooth,
      )

    val animated =
      resolveEditorSurfacePreparation(
        editor = editor,
        scrollFrame = frame,
        currentScroll = 0f,
        bringIntoViewRequest = request,
        smoothScrollEnabled = true,
      )
    val reducedMotion =
      resolveEditorSurfacePreparation(
        editor = editor,
        scrollFrame = frame,
        currentScroll = 0f,
        bringIntoViewRequest = request,
        smoothScrollEnabled = false,
      )

    assertFalse(2 in animated!!.requiredPages)
    assertTrue(2 in reducedMotion!!.requiredPages)
  }

  @Test
  fun `surface planning clamps to a candidate document shorter than the current scroll`() =
    runTest {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val baseFrame = frame()
      val candidateFrame =
        baseFrame.copy(
          state =
            baseFrame.state.copy(
              version = 2L,
              pageSizes = listOf(PageSize(width = 600f, height = 1_000f)),
            )
        )

      val preparation =
        resolveAnchoredEditorSurfacePreparation(
          editor = editor,
          scrollFrame = candidateFrame,
          currentScrollOffset = Offset(x = 0f, y = 90_000f),
          bringIntoViewRequest = null,
          anchorState = EditorViewportAnchorState(),
          publishedBundle =
            PublishedBundle(snapshot = baseFrame.state.copy(version = 1L), frames = emptyMap()),
        )

      val resolved = requireNotNull(preparation)
      assertEquals(setOf(0), resolved.requiredPages)
      assertEquals(
        EditorViewportAnchorPublication.Ready(
          scrollOffset = Offset(x = 0f, y = resolved.maximumScrollY),
          geometry = null,
          attachmentAchieved = false,
        ),
        resolved.anchorPublication,
      )
    }

  @Test
  fun `surface planning reads long document page sizes linearly`() = runTest {
    val pageCount = 128
    val pageSizes = CountingPageSizes(List(pageCount) { PageSize(width = 600f, height = 1_000f) })
    val baseFrame = frame()
    val longDocumentFrame =
      baseFrame.copy(
        state = baseFrame.state.copy(pageSizes = pageSizes),
        editorBounds =
          EditorBoundsInContainer(x = 0f, y = 0f, width = 600f, height = pageCount * 1_000f),
      )

    val preparation =
      resolveEditorSurfacePreparation(
        editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)),
        scrollFrame = longDocumentFrame,
        currentScroll = 0f,
        bringIntoViewRequest = null,
      )

    assertEquals(setOf(0), preparation?.requiredPages)
    assertTrue(
      pageSizes.readCount <= pageCount * 5,
      "Expected linear page-size reads, but read ${pageSizes.readCount} values for $pageCount pages",
    )
  }

  private fun frame(): EditorScrollFrame {
    val visibleArea = EditorVisibleArea(viewport = Size(width = 600f, height = 400f))
    return EditorScrollFrame(
      state =
        EditorState(
          version = 2L,
          cursor = null,
          selection = null,
          pageSizes = List(4) { PageSize(width = 600f, height = 1_000f) },
          externalElements = emptyList(),
          rootAttrs = null,
          rootModifiers = null,
          ime = null,
        ),
      layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea = visibleArea),
      headerHeight = 0f,
      density = 1f,
      editorBounds = EditorBoundsInContainer(x = 0f, y = 0f, width = 600f, height = 4_000f),
    )
  }

  private fun frameWithTrackedItem(id: String): EditorScrollFrame {
    val frame = frame()
    val anchor = Position(node = "1:1", offset = 0, affinity = Affinity.Downstream)
    val head = Position(node = "1:1", offset = 1, affinity = Affinity.Downstream)
    return frame.copy(
      state =
        frame.state.copy(
          trackedRanges =
            listOf(
              TrackedRange(
                id = id,
                group = "comment-active",
                anchor = anchor,
                head = head,
                metadata = "",
                rects =
                  listOf(
                    PageRect(pageIdx = 2, rect = Rect(x = 0f, y = 100f, width = 1f, height = 20f))
                  ),
                text = "comment",
              )
            )
        )
    )
  }

  private class CountingPageSizes(private val values: List<PageSize>) : AbstractList<PageSize>() {
    var readCount = 0
      private set

    override val size: Int
      get() = values.size

    override fun get(index: Int): PageSize {
      readCount += 1
      return values[index]
    }
  }
}
