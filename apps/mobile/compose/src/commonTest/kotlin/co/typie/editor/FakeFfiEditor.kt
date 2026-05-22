package co.typie.editor

import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.PointerStyle
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.Tri

internal class FakeFfiEditor(
  var onTick: () -> List<EditorEvent> = { emptyList() },
  var cursorProvider: () -> CursorMetrics? = { null },
  var selectionProvider: () -> Selection = { EmptySelection },
  var rootAttrsProvider: () -> PlainRootNode = { EmptyRootAttrs },
  var rootModifiersProvider: () -> List<EditorModifier> = { emptyList() },
  var modifierStateProvider: () -> ModifierState = { EmptyModifierState },
  var blockStateProvider: () -> BlockState = {
    BlockState(ancestors = emptyList(), nodes = emptyList())
  },
  var pageSizesProvider: () -> List<Size> = { emptyList() },
  var externalElementsProvider: () -> List<ExternalElement> = { emptyList() },
  var imeProvider: (Int, Int) -> Ime = { _, _ -> EmptyIme },
  var selectionHitProvider: (Int, Float, Float) -> Boolean = { _, _, _ -> false },
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

  override fun selection(): Selection = selectionProvider()

  override fun rootAttrs(): PlainRootNode = rootAttrsProvider()

  override fun rootModifiers(): List<EditorModifier> = rootModifiersProvider()

  override fun modifierState(): ModifierState = modifierStateProvider()

  override fun blockState(): BlockState = blockStateProvider()

  override fun interactiveHitTest(page: Int, x: Float, y: Float): InteractiveHit? = null

  override fun selectionEndpoints(): SelectionEndpoints? = null

  override fun selectionHitTest(page: Int, x: Float, y: Float): Boolean =
    selectionHitProvider(page, x, y)

  override fun pointerStyle(page: Int, x: Float, y: Float, readOnly: Boolean): PointerStyle =
    PointerStyle.Default

  override fun pageSizes(): List<Size> = pageSizesProvider()

  override fun externalElements(): List<ExternalElement> = externalElementsProvider()

  override fun ime(beforeLimit: Int, afterLimit: Int): Ime = imeProvider(beforeLimit, afterLimit)

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

  override fun receiveRemoteChangeset(payload: ByteArray) = Unit

  override fun localChangesetsSince(remoteHeadsPayload: ByteArray): ByteArray = ByteArray(0)

  override fun currentHeads(): ByteArray = ByteArray(0)

  private companion object {
    val EmptyPosition = Position(nodeId = "", offset = 0)
    val EmptySelection = Selection(anchor = EmptyPosition, head = EmptyPosition)
    val EmptyRootAttrs = PlainRootNode(layoutMode = LayoutMode.Continuous(maxWidth = 0))
    val EmptyIme = Ime(text = "", windowStart = 0, selection = ImeRange(0, 0), composing = null)
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
