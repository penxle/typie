package co.typie.editor

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.ChangesetEntry
import co.typie.editor.ffi.CharacterCounts
import co.typie.editor.ffi.ClipboardPayload
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.HistoryTag
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.LinkRect
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.MissingChangesets
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.PartitionedChangesets
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.PlainNodeEntry
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.PointerStyle
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.SearchOptions
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StablePosition
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import co.typie.editor.ffi.TrackedRangeHit
import co.typie.editor.ffi.Tri

internal class FakeFfiEditor(
  var onTick: () -> List<EditorEvent> = { emptyList() },
  var canProvider: (Message) -> Boolean = { false },
  var cursorProvider: () -> CursorMetrics? = { null },
  var placeholderProvider: () -> PlaceholderMetrics? = { null },
  var selectionProvider: () -> Selection? = { EmptySelection },
  var rootAttrsProvider: () -> PlainRootNode = { EmptyRootAttrs },
  var rootModifiersProvider: () -> List<EditorModifier> = { emptyList() },
  var modifierStateProvider: () -> ModifierState = { EmptyModifierState },
  var blockStateProvider: () -> BlockState = {
    BlockState(ancestors = emptyList(), nodes = emptyList(), intersectingNodes = emptyList())
  },
  var characterCountsProvider: () -> CharacterCounts = { EmptyCharacterCounts },
  var pageSizesProvider: () -> List<Size> = { emptyList() },
  var externalElementsProvider: () -> List<ExternalElement> = { emptyList() },
  var tableOverlaysProvider: () -> List<TableOverlay> = { emptyList() },
  var imeProvider: (Int, Int) -> Ime? = { _, _ -> EmptyIme },
  var lastHistoryTagProvider: () -> HistoryTag? = { null },
  var selectionHitProvider: (Int, Float, Float) -> Boolean = { _, _, _ -> false },
  var cursorHitProvider: (Int, Float, Float) -> Boolean = { _, _, _ -> false },
  var interactiveHitProvider: (Int, Float, Float) -> InteractiveHit? = { _, _, _ -> null },
  var selectionEndpointsProvider: () -> SelectionEndpoints? = { null },
  var trackedRangesProvider: (String?) -> List<TrackedRange> = { emptyList() },
  var trackedRangesContainingPositionProvider: (Position, String?) -> List<TrackedRangeEndpoints> =
    { _, _ ->
      emptyList()
    },
  var detachSurfaceProvider: (Int) -> Unit = {},
  var renderSurfaceProvider: (Int) -> Boolean = { true },
) : co.typie.editor.ffi.Editor {
  data class SurfaceResizeCall(
    val page: Int,
    val width: Double,
    val height: Double,
    val scaleFactor: Double,
  )

  val enqueued = mutableListOf<Message>()
  var tickCount: Int = 0
  var renderCount: Int = 0
  var lastRenderedPage: Int? = null
  val resizeCalls = mutableListOf<SurfaceResizeCall>()
  val surfaceEvents = mutableListOf<String>()
  var trackedRangesCallCount: Int = 0
  var trackedRangesContainingPositionCallCount: Int = 0
  var placeholderCallCount: Int = 0
  val insertedTemplateFragments = mutableListOf<ByteArray>()
  val attached = mutableSetOf<Int>()

  override fun enqueue(message: Message) {
    enqueued += message
  }

  override fun can(message: Message): Boolean = canProvider(message)

  override fun lastHistoryTag(): HistoryTag? = lastHistoryTagProvider()

  override fun tick(): List<EditorEvent> {
    tickCount += 1
    return onTick()
  }

  override fun cursor(): CursorMetrics? = cursorProvider()

  override fun placeholder(): PlaceholderMetrics? {
    placeholderCallCount += 1
    return placeholderProvider()
  }

  override fun selection(): Selection? = selectionProvider()

  override fun rootAttrs(): PlainRootNode = rootAttrsProvider()

  override fun rootModifiers(): List<EditorModifier> = rootModifiersProvider()

  override fun modifierState(): ModifierState = modifierStateProvider()

  override fun modifierSpanSelection(pos: Position, modifierType: ModifierType): Selection? = null

  override fun blockState(): BlockState = blockStateProvider()

  override fun characterCounts(): CharacterCounts = characterCountsProvider()

  override fun copySelection(): ClipboardPayload? = null

  override fun interactiveHitTest(page: Int, x: Float, y: Float): InteractiveHit? =
    interactiveHitProvider(page, x, y)

  override fun pageLinkRects(page: Int): List<LinkRect> = emptyList()

  override fun linkRects(): List<LinkRect> = emptyList()

  override fun linkHitTest(page: Int, x: Float, y: Float): LinkRect? = null

  override fun selectionEndpoints(): SelectionEndpoints? = selectionEndpointsProvider()

  override fun selectionHitTest(page: Int, x: Float, y: Float): Boolean =
    selectionHitProvider(page, x, y)

  override fun cursorHitTest(page: Int, x: Float, y: Float): Boolean = cursorHitProvider(page, x, y)

  override fun pointerStyle(page: Int, x: Float, y: Float, readOnly: Boolean): PointerStyle =
    PointerStyle.Default

  override fun pageSizes(): List<Size> = pageSizesProvider()

  override fun pageBackingSizes(): List<Size> = pageSizesProvider()

  override fun externalElements(): List<ExternalElement> = externalElementsProvider()

  override fun pageExternalElements(page: Int): List<ExternalElement> = emptyList()

  override fun tableOverlays(): List<TableOverlay> = tableOverlaysProvider()

  override fun pageTableOverlays(page: Int): List<TableOverlay> = emptyList()

  override fun ime(beforeLimit: Int, afterLimit: Int): Ime? = imeProvider(beforeLimit, afterLimit)

  override fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
  ) {
    surfaceEvents += "attach:$page:$handle"
    attached += page
  }

  override fun detachSurface(page: Int) {
    detachSurfaceProvider(page)
    surfaceEvents += "detach:$page"
    attached -= page
  }

  override fun invalidateSurface(page: Int) = Unit

  override fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) {
    surfaceEvents += "resize:$page:$width:$height:$scaleFactor"
    resizeCalls += SurfaceResizeCall(page, width, height, scaleFactor)
  }

  override fun renderSurface(page: Int): Boolean {
    surfaceEvents += "render:$page"
    renderCount += 1
    lastRenderedPage = page
    return renderSurfaceProvider(page)
  }

  override fun inspectState(options: InspectStateOptions?): String = ""

  override fun inspectStateAsMacro(): String = ""

  override fun receiveRemoteChangeset(payload: ByteArray) = Unit

  override fun localChangesetsSince(remoteHeadsPayload: ByteArray): ByteArray = ByteArray(0)

  override fun changesetIds(): List<String> = emptyList()

  override fun missingChangesetsTolerant(remoteHeadsPayload: ByteArray): MissingChangesets =
    MissingChangesets(bytes = emptyList(), withheld = 0)

  override fun partitionRemoteChangesets(payload: ByteArray): PartitionedChangesets =
    PartitionedChangesets(ready = emptyList(), blocked = emptyList())

  override fun splitChangesets(payload: ByteArray): List<ChangesetEntry> = emptyList()

  override fun currentHeads(): ByteArray = ByteArray(0)

  override fun setDoc(plain: PlainDoc) = Unit

  override fun insertTemplateFragment(changesets: ByteArray) {
    insertedTemplateFragments += changesets.copyOf()
  }

  override fun materializeAt(heads: ByteArray, sweepTombstones: List<String>): PlainDoc =
    EmptyPlainDoc

  override fun freezeSelection(selection: Selection): StableSelection? =
    // 2 == editor-state STABLE_SELECTION_WIRE_VERSION (StableSelection wire v2).
    StableSelection(version = 2, anchor = EmptyStablePosition, head = EmptyStablePosition)

  override fun findMatches(query: String, options: SearchOptions?): List<Selection> = emptyList()

  override fun trackedRanges(group: String?): List<TrackedRange> {
    trackedRangesCallCount += 1
    return trackedRangesProvider(group)
  }

  override fun trackedRangesContainingPosition(
    position: Position,
    group: String?,
  ): List<TrackedRangeEndpoints> {
    trackedRangesContainingPositionCallCount += 1
    return trackedRangesContainingPositionProvider(position, group)
  }

  override fun exportPageVector(page: Int, scaleFactor: Double): ByteArray = ByteArray(0)

  override fun trackedRangesAt(
    page: Int,
    x: Float,
    y: Float,
    group: String?,
  ): List<TrackedRangeHit> = emptyList()

  override fun proseText(): String = ""

  override fun proseToSelection(start: Int, end: Int): Selection? = null

  private companion object {
    val EmptyPosition = Position(node = "", offset = 0, affinity = Affinity.Downstream)
    val EmptyStablePosition =
      StablePosition(chain = emptyList(), child = null, affinity = Affinity.Downstream)
    val EmptySelection = Selection(anchor = EmptyPosition, head = EmptyPosition)
    val EmptyRootAttrs = PlainRootNode(layoutMode = LayoutMode.Continuous(maxWidth = 0))
    val EmptyPlainDoc =
      PlainDoc(
        root =
          PlainNodeEntry(
            node = PlainNode.Root(layoutMode = LayoutMode.Continuous(maxWidth = 0)),
            modifiers = emptyMap(),
            children = emptyList(),
          )
      )
    val EmptyIme = Ime(text = "", windowStart = 0, selection = ImeRange(0, 0), composing = null)
    val EmptyCharacterCounts =
      CharacterCounts(
        docWithWhitespace = 0,
        docWithoutWhitespace = 0,
        docWithoutWhitespaceAndPunctuation = 0,
        selectionWithWhitespace = 0,
        selectionWithoutWhitespace = 0,
        selectionWithoutWhitespaceAndPunctuation = 0,
      )
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
        effectiveBold = Tri.Absent,
        cellBackgroundColor = null,
      )
  }
}
