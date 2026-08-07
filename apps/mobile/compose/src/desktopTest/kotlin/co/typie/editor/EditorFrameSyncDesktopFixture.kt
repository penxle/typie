package co.typie.editor

import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.input.key.Key as ComposeKey
import androidx.compose.ui.test.ExperimentalTestApi
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Editor as FfiEditor
import co.typie.editor.ffi.GraphIngest
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.PlainNodeEntry
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.Viewport
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.scroll.updateNowWithBringIntoView
import co.typie.editor.sync.DocumentEditorLoad
import co.typie.editor.sync.createTestDocumentEditingSession
import co.typie.editor.sync.ws.DocumentSyncBaseline
import co.typie.editor.viewport.EditorViewportState
import java.io.File
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestCoroutineScheduler

@OptIn(ExperimentalTestApi::class)
internal class FrameSyncFixture(
  continuous: Boolean = false,
  initialDoc: PlainDoc? = null,
  viewportHeight: Float = ViewportHeight,
) {
  val continuationScheduler = TestCoroutineScheduler()
  private val continuationDispatcher = StandardTestDispatcher(continuationScheduler)
  val scope = CoroutineScope(SupervisorJob() + continuationDispatcher)
  val layoutSpec: EditorDocumentLayoutSpec =
    if (continuous) {
      EditorDocumentLayoutSpec.Continuous(maxWidth = PageWidth)
    } else {
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = PageWidth,
        pageHeight = PageHeight,
        pageMarginTop = PageMargin,
        pageMarginBottom = PageMargin,
        pageMarginLeft = PageMargin,
        pageMarginRight = PageMargin,
      )
    }
  val visibleArea = EditorVisibleArea(viewport = Size(ViewportWidth, viewportHeight))
  val autoScrollPolicy =
    resolveEditorAutoScrollPolicy(visibleArea = visibleArea, baseBottomSpace = PageMargin)
  val viewportState = EditorViewportState()
  val uiState = EditorUiState()
  val bringIntoViewRequests = EditorBringIntoViewRequests()
  private var drawSequence = 0L
  private val drawJournal = mutableListOf<DrawnPresentation>()
  val forceDrawTick = mutableIntStateOf(0)
  @Volatile var lastDrawnPresentation: DrawnPresentation? = null
  val editor: Editor
  private val session: DocumentEditingSession
  val runtime: EditorRuntime
  val load: DocumentEditorLoad

  init {
    configureRenderBufferLibrary()
    editor =
      kotlinx.coroutines.runBlocking {
        Editor.createFromDoc(
          doc =
            initialDoc ?: if (continuous) emptyContinuousDocument() else emptyPaginatedDocument(),
          viewport = Viewport(ViewportWidth, viewportHeight, 1.0),
          scope = scope,
          dispatcher = Dispatchers.Default.limitedParallelism(1),
        )
      }
    editor.updateNow {
      enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = PageMargin, y = PageMargin)))
    }
    session = createTestDocumentEditingSession(editor, scope)
    runtime = EditorRuntime(scope)
    runtime.attach(session)
    load =
      DocumentEditorLoad(
        ingest = UnusedGraphIngest,
        initialBaseline = DocumentSyncBaseline("", ByteArray(0), ByteArray(0)),
        pending = emptyList(),
        parentScope = scope,
        onEditorError = { _, _ -> },
      )
  }

  @Synchronized
  fun recordDraw(
    snapshot: EditorState,
    scrollY: Float,
    cursorNativeFrameRevision: Long?,
    testDrawTick: Int,
  ) {
    val cursorPage = snapshot.cursor?.pageIdx
    val draw =
      DrawnPresentation(
        sequence = ++drawSequence,
        snapshot = snapshot,
        scrollY = scrollY,
        cursorNativeFrameRevision = cursorNativeFrameRevision,
        editorBoundsY = uiState.editorBoundsInContainer.y,
        cursorPageOffsetY =
          cursorPage?.let {
            uiState.resolveViewportTransform(snapshot.pageSizes).pageOffsets[it]?.y
          },
        testDrawTick = testDrawTick,
      )
    drawJournal += draw
    lastDrawnPresentation = draw
  }

  @Synchronized
  fun drawsAfter(sequence: Long): List<DrawnPresentation> = drawJournal.filter {
    it.sequence > sequence
  }

  @Synchronized fun latestDrawSequence(): Long = drawSequence

  fun forceNextDraw() {
    forceDrawTick.intValue += 1
  }

  fun scrollFrame(state: EditorState): EditorScrollFrame =
    EditorScrollFrame(
      state = state,
      layoutSpec = layoutSpec,
      displayZoom = 1f,
      visibleArea = visibleArea,
      autoScrollPolicy = autoScrollPolicy,
      headerHeight = 0f,
      density = 1f,
      editorBounds = uiState.editorBoundsInContainer,
    )

  fun moveToLastOnePageState(test: androidx.compose.ui.test.ComposeUiTest) {
    repeat(MaxBoundarySearchSteps) {
      val update =
        assertNotNull(
          editor.updateNowWithBringIntoView(bringIntoViewRequests) {
            enqueue(Message.Key(KeyEvent(Key.Enter)))
            bringIntoView(
              EditorBringIntoViewTarget.CurrentSelectionHead,
              policy = EditorBringIntoViewPolicy.Typewriter,
            )
          }
        )
      test.waitUntil(timeoutMillis = 10_000) {
        (editor.publishedRevision ?: -1L) >= update.revision
      }
      test.waitForIdle()
      if (editor.publishedState.pageSizes.size > 1) {
        val onePageUpdate =
          assertNotNull(
            editor.updateNowWithBringIntoView(bringIntoViewRequests) {
              enqueue(Message.Key(KeyEvent(Key.Backspace)))
              bringIntoView(
                EditorBringIntoViewTarget.CurrentSelectionHead,
                policy = EditorBringIntoViewPolicy.Typewriter,
              )
            }
          )
        test.waitUntil(timeoutMillis = 10_000) {
          (editor.publishedRevision ?: -1L) >= onePageUpdate.revision
        }
        test.waitForIdle()
        assertEquals(1, editor.publishedState.pageSizes.size)
        return
      }
    }
    val state = editor.publishedState
    error(
      "page boundary was not reached within $MaxBoundarySearchSteps Enter presses: " +
        "version=${state.version} pages=${state.pageSizes} cursor=${state.cursor} " +
        "selection=${state.selection} ime=${state.ime} root=${state.rootAttrs}"
    )
  }

  fun prepareLongDocument(test: androidx.compose.ui.test.ComposeUiTest) {
    val update =
      assertNotNull(
        editor.updateNowWithBringIntoView(bringIntoViewRequests) {
          repeat(LongDocumentParagraphCount) { enqueue(Message.Key(KeyEvent(Key.Enter))) }
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.Typewriter,
          )
        }
      )
    test.waitUntil(timeoutMillis = 10_000) {
      val bundle = editor.publishedBundle ?: return@waitUntil false
      val cursorPage = bundle.snapshot.cursor?.pageIdx ?: return@waitUntil false
      bundle.snapshot.version >= update.revision && bundle.frames.containsKey(cursorPage)
    }
    test.waitForIdle()
    assertTrue(editor.publishedState.pageSizes.size >= 3)

    val startUpdate =
      assertNotNull(
        editor.updateNowWithBringIntoView(bringIntoViewRequests) {
          enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = PageMargin, y = PageMargin)))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.CursorGuard,
          )
        }
      )
    test.waitUntil(timeoutMillis = 10_000) {
      (editor.publishedRevision ?: -1L) >= startUpdate.revision
    }
    test.waitForIdle()
    viewportState.scrollToY(0f, isAutoScroll = false)
  }

  fun resetLongDocumentStart(test: androidx.compose.ui.test.ComposeUiTest) {
    val startUpdate =
      assertNotNull(
        editor.updateNowWithBringIntoView(bringIntoViewRequests) {
          enqueue(Message.Selection(SelectionOp.SetAt(page = 0, x = PageMargin, y = PageMargin)))
        }
      )
    viewportState.scrollToY(0f, isAutoScroll = false)
    test.mainClock.autoAdvance = true
    try {
      test.waitUntil(timeoutMillis = 10_000) {
        (editor.publishedRevision ?: -1L) >= startUpdate.revision &&
          editor.publishedState.cursor?.pageIdx == 0
      }
      test.waitForIdle()
    } finally {
      test.mainClock.autoAdvance = false
    }
    viewportState.scrollToY(0f, isAutoScroll = false)
  }

  fun moveToTwoPageEnd(test: androidx.compose.ui.test.ComposeUiTest) {
    moveToLastOnePageState(test)
    val twoPageUpdate =
      assertNotNull(
        editor.updateNowWithBringIntoView(bringIntoViewRequests) {
          enqueue(Message.Key(KeyEvent(Key.Enter)))
          bringIntoView(
            EditorBringIntoViewTarget.CurrentSelectionHead,
            policy = EditorBringIntoViewPolicy.Typewriter,
          )
        }
      )
    test.waitUntil(timeoutMillis = 10_000) {
      (editor.publishedRevision ?: -1L) >= twoPageUpdate.revision &&
        editor.publishedBundle?.frames?.get(1) != null
    }
    test.waitForIdle()
    assertEquals(2, editor.publishedState.pageSizes.size)
  }

  fun close() {
    load.close()
    runtime.clear()
    scope.cancel()
  }
}

internal enum class RepeatKey(val key: ComposeKey) {
  Down(ComposeKey.DirectionDown),
  Up(ComposeKey.DirectionUp),
  Enter(ComposeKey.Enter),
  Backspace(ComposeKey.Backspace),
}

internal data class DrawnPresentation(
  val sequence: Long,
  val snapshot: EditorState,
  val scrollY: Float,
  val cursorNativeFrameRevision: Long?,
  val editorBoundsY: Float,
  val cursorPageOffsetY: Float?,
  val testDrawTick: Int,
)

internal data class DesktopFrameSample(
  val phaseMillis: Long,
  val displayFrame: Int,
  val repeatKey: RepeatKey?,
  val presentation: DrawnPresentation,
  val drawPassCount: Int,
  val cursorDocumentY: Float,
  val highlightDocumentY: Float,
  val expectedCursorY: Float,
  val actualCursorY: Float?,
  val expectedHighlightY: Float,
  val actualHighlightY: Float?,
)

internal data class CapturedDesktopFrame(val sample: DesktopFrameSample, val image: ImageBitmap)

internal const val RootTag = "editor-frame-sync-root"
internal const val ViewportWidth = 240f
internal const val ViewportHeight = 140f
internal const val PageWidth = 200f
internal const val PageHeight = 180f
internal const val PageMargin = 20f
internal const val HighlightSampleInset = 8f
internal const val CursorScanRadius = 4
internal const val RepeatIntervalMillis = 33L
internal const val RepeatFramesPerLeg = 8
internal const val RepeatCyclesPerPhase = 3
internal const val EnterBackspaceRepeatsPerLeg = 60
internal const val EnterBackspaceCyclesPerPhase = 3
internal const val MaximumPublicationStallMillis = 250L
internal const val CoordinateTolerancePx = 2f
internal const val PixelBandHeightTolerance = 2
internal const val ColorChannelTolerance = 2f / 255f
internal val RepeatStartPhasesMillis = listOf(0L, 16L)
private const val MaxBoundarySearchSteps = 32
internal const val LongDocumentParagraphCount = 24

private object UnusedGraphIngest : GraphIngest {
  override fun appendChunk(data: ByteArray) = error("unused")

  override fun totalBytes(): Long = 0L

  override fun abort() = Unit

  override fun finish(viewport: Viewport): FfiEditor = error("unused")

  override fun finishWithPending(pendingEncoded: ByteArray, viewport: Viewport): FfiEditor =
    error("unused")
}

private fun emptyPaginatedDocument(): PlainDoc =
  emptyDocument(
    LayoutMode.Paginated(
      pageWidth = PageWidth.toInt(),
      pageHeight = PageHeight.toInt(),
      pageMarginTop = PageMargin.toInt(),
      pageMarginBottom = PageMargin.toInt(),
      pageMarginLeft = PageMargin.toInt(),
      pageMarginRight = PageMargin.toInt(),
    )
  )

private fun emptyContinuousDocument(): PlainDoc =
  emptyDocument(LayoutMode.Continuous(maxWidth = PageWidth.toInt()))

internal fun continuousDocumentWithOffscreenTable(): PlainDoc {
  val document = emptyContinuousDocument()
  return document.copy(
    root =
      document.root.copy(
        children =
          buildList {
            add(paragraph("selectable word"))
            repeat(80) { add(paragraph("")) }
            add(
              PlainNodeEntry(
                node = PlainNode.Table(),
                modifiers = emptyMap(),
                children =
                  listOf(
                    PlainNodeEntry(
                      node = PlainNode.TableRow,
                      modifiers = emptyMap(),
                      children =
                        listOf(
                          PlainNodeEntry(
                            node = PlainNode.TableCell(colWidth = null, backgroundColor = null),
                            modifiers = emptyMap(),
                            children = listOf(paragraph("offscreen table")),
                          )
                        ),
                    )
                  ),
              )
            )
          }
      )
  )
}

private fun paragraph(text: String): PlainNodeEntry =
  PlainNodeEntry(
    node = PlainNode.Paragraph,
    modifiers = emptyMap(),
    children =
      listOf(
        PlainNodeEntry(node = PlainNode.Text(text), modifiers = emptyMap(), children = emptyList())
      ),
  )

private fun emptyDocument(layoutMode: LayoutMode): PlainDoc =
  PlainDoc(
    root =
      PlainNodeEntry(
        node = PlainNode.Root(layoutMode),
        modifiers =
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
          ),
        children = listOf(paragraph("")),
      )
  )

private fun configureRenderBufferLibrary() {
  if (System.getProperty("jna.library.path") != null) return

  val repository =
    generateSequence(File(System.getProperty("user.dir"))) { it.parentFile }
      .firstOrNull { File(it, "Cargo.toml").isFile } ?: error("Typie repository root not found")
  val host =
    when (System.getProperty("os.arch")) {
      "aarch64" -> "aarch64-apple-darwin"
      "x86_64" -> "x86_64-apple-darwin"
      else -> error("Unsupported desktop test architecture: ${System.getProperty("os.arch")}")
    }
  val directory = File(repository, "target/$host/release-uniffi")
  check(File(directory, "libeditor_ffi.dylib").isFile) {
    "Desktop editor FFI is not built; run `just -f crates/editor-ffi/justfile desktop`"
  }
  System.setProperty("jna.library.path", directory.absolutePath)
}
