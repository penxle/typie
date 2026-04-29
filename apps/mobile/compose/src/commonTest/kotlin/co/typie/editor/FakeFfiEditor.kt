package co.typie.editor

import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.RootNode
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.Tri

internal class FakeFfiEditor(
  var onTick: () -> List<EditorEvent> = { emptyList() },
  var cursorProvider: () -> CursorMetrics? = { null },
  var selectionProvider: () -> Selection? = { null },
  var rootAttrsProvider: () -> RootNode? = { null },
  var modifierStateProvider: () -> ModifierState = { EmptyModifierState },
  var blockStateProvider: () -> BlockState = {
    BlockState(ancestors = emptyList(), nodes = emptyList())
  },
  var pageSizesProvider: () -> List<Size> = { emptyList() },
  var imeProvider: (Int, Int) -> Ime? = { _, _ -> null },
) : co.typie.editor.ffi.Editor {
  val enqueued = mutableListOf<Message>()
  var tickCount: Int = 0
  var renderCount: Int = 0
  var lastRenderedPage: Int? = null
  val attached = mutableSetOf<Int>()

  override fun enqueue(message: Message) {
    enqueued += message
  }

  override fun tick(): List<EditorEvent> {
    tickCount += 1
    return onTick()
  }

  override fun cursor(): CursorMetrics? = cursorProvider()

  override fun selection(): Selection =
    selectionProvider() ?: error("selection not set in FakeFfiEditor")

  override fun rootAttrs(): RootNode =
    rootAttrsProvider() ?: error("rootAttrs not set in FakeFfiEditor")

  override fun modifierState(): ModifierState = modifierStateProvider()

  override fun blockState(): BlockState = blockStateProvider()

  override fun pageSizes(): List<Size> = pageSizesProvider()

  override fun ime(beforeLimit: Int, afterLimit: Int): Ime =
    imeProvider(beforeLimit, afterLimit) ?: error("ime not set in FakeFfiEditor")

  override fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
  ) {
    attached += page
  }

  override fun detachSurface(page: Int) {
    attached -= page
  }

  override fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) = Unit

  override fun renderSurface(page: Int) {
    renderCount += 1
    lastRenderedPage = page
  }

  override fun inspectState(options: InspectStateOptions?): String = ""

  override fun inspectStateAsMacro(): String = ""

  private companion object {
    val EmptyModifierState =
      ModifierState(
        bold = Tri.Absent,
        italic = Tri.Absent,
        underline = Tri.Absent,
        strikethrough = Tri.Absent,
        fontSize = Tri.Absent,
        fontFamily = Tri.Absent,
        fontWeight = Tri.Absent,
        textColor = Tri.Absent,
        backgroundColor = Tri.Absent,
        letterSpacing = Tri.Absent,
        link = Tri.Absent,
        ruby = Tri.Absent,
        lineHeight = Tri.Absent,
        blockGap = Tri.Absent,
        paragraphIndent = Tri.Absent,
        alignment = Tri.Absent,
      )
  }
}
