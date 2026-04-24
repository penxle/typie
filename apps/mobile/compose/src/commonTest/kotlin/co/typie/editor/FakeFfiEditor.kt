package co.typie.editor

import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size

internal class FakeFfiEditor(
  var onTick: () -> List<EditorEvent> = { emptyList() },
  var cursorProvider: () -> CursorMetrics? = { null },
  var selectionProvider: () -> Selection? = { null },
  var documentAttrsProvider: () -> DocumentAttrs? = { null },
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

  override fun documentAttrs(): DocumentAttrs =
    documentAttrsProvider() ?: error("documentAttrs not set in FakeFfiEditor")

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
}
