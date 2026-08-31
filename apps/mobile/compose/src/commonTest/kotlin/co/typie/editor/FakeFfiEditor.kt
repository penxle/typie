package co.typie.editor

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.CapturedViewportAnchor
import co.typie.editor.ffi.ChangesetEntry
import co.typie.editor.ffi.CharacterCounts
import co.typie.editor.ffi.ClipboardPayload
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExpansionAffordances
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.HistoryTag
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.InteractiveRegion
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.LinkRect
import co.typie.editor.ffi.ListAffordances
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.MissingChangesets
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.PartitionedChangesets
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.PlainNodeEntry
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.PointerStyle
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.RequestId
import co.typie.editor.ffi.RequestOutcome
import co.typie.editor.ffi.ResourceUpdate
import co.typie.editor.ffi.Revision
import co.typie.editor.ffi.SearchOptions
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionKind
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StablePosition
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TickResult
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import co.typie.editor.ffi.TrackedRangeHit
import co.typie.editor.ffi.Tri
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.ffi.ViewportAnchorResolution

internal class FakeFfiEditor(
  var onTick: () -> List<EditorEvent> = { emptyList() },
  var cursorProvider: () -> CursorMetrics? = { null },
  var placeholderProvider: () -> PlaceholderMetrics? = { null },
  var selectionProvider: () -> Selection? = { EmptySelection },
  var rootAttrsProvider: () -> PlainRootNode = { EmptyRootAttrs },
  var rootModifiersProvider: () -> List<EditorModifier> = { emptyList() },
  var modifierStateProvider: () -> ModifierState = { EmptyModifierState },
  var blockStateProvider: () -> BlockState = {
    BlockState(
      ancestors = emptyList(),
      nodes = emptyList(),
      intersectingNodes = emptyList(),
      list =
        ListAffordances(
          toggleBullet = false,
          toggleOrdered = false,
          indent = false,
          outdent = false,
        ),
      expansion =
        ExpansionAffordances(word = false, sentence = false, paragraph = false, all = false),
    )
  },
  var characterCountsProvider: () -> CharacterCounts = { EmptyCharacterCounts },
  var pageSizesProvider: () -> List<Size> = { emptyList() },
  var pageBackingSizesProvider: () -> List<Size> = pageSizesProvider,
  var externalElementsProvider: () -> List<ExternalElement> = { emptyList() },
  var tableOverlaysProvider: () -> List<TableOverlay> = { emptyList() },
  var imeProvider: (Int, Int) -> Ime? = { _, _ -> EmptyIme },
  var lastHistoryTagProvider: () -> HistoryTag? = { null },
  var currentHeadsProvider: () -> ByteArray = { ByteArray(0) },
  var localChangesetsSinceProvider: (ByteArray) -> ByteArray = { ByteArray(0) },
  var selectionHitProvider: (Int, Float, Float) -> Boolean = { _, _, _ -> false },
  var cursorHitProvider: (Int, Float, Float) -> Boolean = { _, _, _ -> false },
  var interactiveHitProvider: (Int, Float, Float) -> InteractiveHit? = { _, _, _ -> null },
  var selectionHitRectsProvider: () -> List<PageRect> = { emptyList() },
  var cursorHitRectsProvider: () -> List<PageRect> = { emptyList() },
  var interactiveRegionsProvider: () -> List<InteractiveRegion> = { emptyList() },
  var replaceViewportAnchorPresentationProvider: (Revision) -> Boolean = { true },
  var captureSelectionViewportAnchorProvider: (Revision) -> CapturedViewportAnchor? = { null },
  var captureViewportAnchorAtProvider: (Revision, ViewportAnchorPoint) -> CapturedViewportAnchor? =
    { _, _ ->
      null
    },
  var resolveViewportAnchorProvider: (Revision, ViewportAnchor) -> ViewportAnchorResolution =
    { _, _ ->
      ViewportAnchorResolution.Unavailable
    },
  var copySelectionProvider: () -> ClipboardPayload? = { null },
  var proseTextAnnotatedProvider: () -> String = { "" },
  var selectionEndpointsProvider: () -> SelectionEndpoints? = { null },
  var trackedRangesProvider: (String?) -> List<TrackedRange> = { emptyList() },
  var trackedRangesContainingSelectionProvider:
    (Selection, String?) -> List<TrackedRangeEndpoints> =
    { _, _ ->
      emptyList()
    },
  var receiveRemoteChangesetProvider: (ByteArray) -> Unit = {},
  var receiveResourceUpdateProvider: (ResourceUpdate) -> Unit = {},
  var beforeEnqueueRequest: () -> Unit = {},
  var detachSurfaceProvider: (Int) -> Unit = {},
  var renderSurfaceProvider: (Int) -> Boolean = { true },
) : co.typie.editor.ffi.Editor {
  data class EnqueuedRequest(val id: RequestId, val messages: List<Message>)

  data class SurfaceAttachCall(
    val page: Int,
    val width: Double,
    val height: Double,
    val scaleFactor: Double,
  )

  data class SurfaceResizeCall(
    val page: Int,
    val width: Double,
    val height: Double,
    val scaleFactor: Double,
  )

  data class SurfaceRenderCall(val page: Int, val requestedRevision: Revision)

  private sealed interface PendingEntry

  private data class PendingRequest(val request: EnqueuedRequest) : PendingEntry

  private data object ResourceEntry : PendingEntry

  private data object PendingNativeWork : PendingEntry

  val enqueued = mutableListOf<Message>()
  val enqueuedRequests = mutableListOf<EnqueuedRequest>()
  val receivedResourceUpdates = mutableListOf<ResourceUpdate>()
  val tickThroughRequests = mutableListOf<RequestId>()
  val renderCalls = mutableListOf<SurfaceRenderCall>()
  var commandOutcomesProvider:
    (RequestId, List<Message>) -> List<co.typie.editor.ffi.CommandOutcome> =
    { _, messages ->
      List(messages.size) { co.typie.editor.ffi.CommandOutcome.Applied }
    }
  var tickResultProvider: (() -> TickResult?)? = null
  var tickThroughProvider: ((RequestId) -> TickResult)? = null
  var renderFrameProvider: ((Int, Revision) -> FrameKey?)? = null
  var tickWhenIdle: Boolean = false
  var tickCount: Int = 0
  var renderCount: Int = 0
  var lastRenderedPage: Int? = null
  val attachCalls = mutableListOf<SurfaceAttachCall>()
  val resizeCalls = mutableListOf<SurfaceResizeCall>()
  val surfaceEvents = mutableListOf<String>()
  var trackedRangesCallCount: Int = 0
  var trackedRangesContainingSelectionCallCount: Int = 0
  var placeholderCallCount: Int = 0
  val insertedTemplateFragments = mutableListOf<ByteArray>()
  val attached = mutableSetOf<Int>()
  private val pendingEntries = mutableListOf<PendingEntry>()
  private var nextRequestId = 0L
  private var nextRevision = 0L
  private var nextFrameKey = 0L
  private var refreshAllFieldsOnNextTick = false

  override fun enqueueRequest(messages: List<Message>): RequestId {
    beforeEnqueueRequest()
    val request = EnqueuedRequest(id = RequestId(++nextRequestId), messages = messages.toList())
    enqueued += request.messages
    enqueuedRequests += request
    pendingEntries += PendingRequest(request)
    return request.id
  }

  /**
   * Materializes the next fake snapshot through the current request API. The synthetic Initialize
   * request is removed from the command recorder so tests continue to observe only the user action
   * under test.
   */
  fun applySnapshot(editor: Editor): EditorUpdate {
    val requestCount = enqueuedRequests.size
    val tickThroughCount = tickThroughRequests.size
    val messageCount = enqueued.size
    val update =
      try {
        refreshAllFieldsOnNextTick = true
        requireNotNull(editor.updateNow { enqueue(Message.System(SystemEvent.Initialize)) })
      } finally {
        refreshAllFieldsOnNextTick = false
      }
    enqueued.subList(messageCount, enqueued.size).clear()
    enqueuedRequests.subList(requestCount, enqueuedRequests.size).clear()
    tickThroughRequests.subList(tickThroughCount, tickThroughRequests.size).clear()
    return update
  }

  /** Applies and publishes the next fake snapshot through an active zero-target visual host. */
  fun publishSnapshot(editor: Editor): EditorUpdate {
    editor.activateVisualHost(this)
    val update = applySnapshot(editor)
    editor.requestSurfacePages(emptySet())
    requireNotNull(editor.publishIfReady(emptySet())).let(editor::acceptPublication)
    return update
  }

  fun attachSurfaceWithoutFrame(
    editor: Editor,
    page: Int = 0,
    handle: Long = 1L,
    width: Double = 100.0,
    height: Double = 100.0,
  ): SurfaceSessionHandle {
    editor.activateVisualHost(this)
    return editor.attachSurface(
      page = page,
      handle = handle,
      width = width,
      height = height,
      scaleFactor = 1.0,
      wakeDelivery = {},
    )
  }

  override fun receiveResourceUpdate(update: ResourceUpdate) {
    receivedResourceUpdates += update
    if (pendingEntries.lastOrNull() !== ResourceEntry) {
      pendingEntries += ResourceEntry
    }
    receiveResourceUpdateProvider(update)
  }

  override fun lastHistoryTag(): HistoryTag? = lastHistoryTagProvider()

  override fun tick(): TickResult? {
    tickCount += 1
    tickResultProvider?.let {
      return it()
    }

    if (pendingEntries.isEmpty() && !tickWhenIdle) return null
    val entries = pendingEntries.toList()
    pendingEntries.clear()
    return tickResult(entries.requests(), onTick())
  }

  override fun tickThrough(requestId: RequestId): TickResult {
    tickCount += 1
    tickThroughRequests += requestId
    tickThroughProvider?.let {
      return it(requestId)
    }

    val index = pendingEntries.indexOfLast { entry ->
      entry is PendingRequest && entry.request.id == requestId
    }
    check(index >= 0) { "Request ${requestId.value} is not queued" }
    val entries = pendingEntries.take(index + 1)
    repeat(index + 1) { pendingEntries.removeAt(0) }
    return tickResult(entries.requests(), onTick())
  }

  override fun replaceViewportAnchorPresentation(revision: Revision): Boolean =
    replaceViewportAnchorPresentationProvider(revision)

  override fun captureSelectionViewportAnchor(revision: Revision): CapturedViewportAnchor? =
    captureSelectionViewportAnchorProvider(revision)

  override fun captureViewportAnchorAt(
    revision: Revision,
    point: ViewportAnchorPoint,
  ): CapturedViewportAnchor? = captureViewportAnchorAtProvider(revision, point)

  override fun resolveViewportAnchor(
    revision: Revision,
    anchor: ViewportAnchor,
  ): ViewportAnchorResolution = resolveViewportAnchorProvider(revision, anchor)

  override fun cursor(): CursorMetrics? = cursorProvider()

  override fun placeholder(): PlaceholderMetrics? {
    placeholderCallCount += 1
    return placeholderProvider()
  }

  override fun selection(): Selection? = selectionProvider()

  override fun selectionKind(): SelectionKind? = null

  override fun rootAttrs(): PlainRootNode = rootAttrsProvider()

  override fun rootModifiers(): List<EditorModifier> = rootModifiersProvider()

  override fun modifierState(): ModifierState = modifierStateProvider()

  override fun modifierSpanSelection(pos: Position, modifierType: ModifierType): Selection? = null

  override fun blockState(): BlockState = blockStateProvider()

  override fun characterCounts(): CharacterCounts = characterCountsProvider()

  override fun copySelection(): ClipboardPayload? = copySelectionProvider()

  override fun interactiveHitTest(page: Int, x: Float, y: Float): InteractiveHit? =
    interactiveHitProvider(page, x, y)

  override fun pageLinkRects(page: Int): List<LinkRect> = emptyList()

  override fun linkRects(): List<LinkRect> = emptyList()

  override fun linkHitTest(page: Int, x: Float, y: Float): LinkRect? = null

  override fun selectionEndpoints(): SelectionEndpoints? = selectionEndpointsProvider()

  override fun selectionHitTest(page: Int, x: Float, y: Float): Boolean =
    selectionHitProvider(page, x, y)

  override fun selectionHitRects(): List<PageRect> = selectionHitRectsProvider()

  override fun cursorHitTest(page: Int, x: Float, y: Float): Boolean = cursorHitProvider(page, x, y)

  override fun cursorHitRects(): List<PageRect> = cursorHitRectsProvider()

  override fun interactiveRegions(): List<InteractiveRegion> = interactiveRegionsProvider()

  override fun pointerStyle(page: Int, x: Float, y: Float, readOnly: Boolean): PointerStyle =
    PointerStyle.Default

  override fun pageSizes(): List<Size> = pageSizesProvider()

  override fun pageBackingSizes(): List<Size> = pageBackingSizesProvider()

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
    attachCalls += SurfaceAttachCall(page, width, height, scaleFactor)
    attached += page
  }

  override fun detachSurface(page: Int) {
    detachSurfaceProvider(page)
    surfaceEvents += "detach:$page"
    attached -= page
  }

  override fun invalidateSurface(page: Int) {
    surfaceEvents += "invalidate:$page"
  }

  override fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) {
    surfaceEvents += "resize:$page:$width:$height:$scaleFactor"
    resizeCalls += SurfaceResizeCall(page, width, height, scaleFactor)
  }

  override fun renderSurface(page: Int, requestedRevision: Revision): FrameKey? {
    surfaceEvents += "render:$page"
    renderCount += 1
    lastRenderedPage = page
    renderCalls += SurfaceRenderCall(page, requestedRevision)
    val provider = renderFrameProvider
    return if (provider != null) {
      provider(page, requestedRevision)
    } else if (renderSurfaceProvider(page)) {
      FrameKey(++nextFrameKey)
    } else {
      null
    }
  }

  private fun tickResult(requests: List<EnqueuedRequest>, events: List<EditorEvent>): TickResult =
    TickResult(
      revision = Revision(++nextRevision),
      events =
        if (refreshAllFieldsOnNextTick) {
          events + EditorEvent.StateChanged(AllStateFields)
        } else {
          events
        },
      requestOutcomes =
        requests.map { request ->
          RequestOutcome(
            requestId = request.id,
            commandOutcomes = commandOutcomesProvider(request.id, request.messages),
          )
        },
    )

  private fun List<PendingEntry>.requests(): List<EnqueuedRequest> = mapNotNull {
    (it as? PendingRequest)?.request
  }

  override fun inspectState(options: InspectStateOptions?): String = ""

  override fun inspectStateAsMacro(): String = ""

  override fun inspectSelectionAsSliceMacro(): String? = null

  override fun receiveRemoteChangeset(payload: ByteArray) {
    pendingEntries += PendingNativeWork
    receiveRemoteChangesetProvider(payload)
  }

  override fun localChangesetsSince(remoteHeadsPayload: ByteArray): ByteArray =
    localChangesetsSinceProvider(remoteHeadsPayload)

  override fun changesetIds(): List<String> = emptyList()

  override fun missingChangesetsTolerant(remoteHeadsPayload: ByteArray): MissingChangesets =
    MissingChangesets(bytes = emptyList(), withheld = 0)

  override fun partitionRemoteChangesets(payload: ByteArray): PartitionedChangesets =
    PartitionedChangesets(ready = emptyList(), blocked = emptyList())

  override fun splitChangesets(payload: ByteArray): List<ChangesetEntry> = emptyList()

  override fun currentHeads(): ByteArray = currentHeadsProvider()

  override fun setDoc(plain: PlainDoc) = Unit

  override fun insertTemplateFragment(changesets: ByteArray) {
    insertedTemplateFragments += changesets.copyOf()
    pendingEntries += PendingNativeWork
  }

  override fun materializeAt(heads: ByteArray, sweepTombstones: List<String>): PlainDoc =
    EmptyPlainDoc

  override fun freezeSelection(selection: Selection): StableSelection? =
    // 2 == editor-state STABLE_SELECTION_WIRE_VERSION (StableSelection wire v2).
    StableSelection(version = 2, anchor = EmptyStablePosition, head = EmptyStablePosition)

  override fun findMatches(query: String, options: SearchOptions?): List<Selection> = emptyList()

  override fun trackedRange(id: String): TrackedRange? = null

  override fun trackedRanges(group: String?): List<TrackedRange> {
    trackedRangesCallCount += 1
    return trackedRangesProvider(group)
  }

  override fun trackedRangesContainingSelection(
    selection: Selection,
    group: String?,
  ): List<TrackedRangeEndpoints> {
    trackedRangesContainingSelectionCallCount += 1
    return trackedRangesContainingSelectionProvider(selection, group)
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

  override fun proseTextAnnotated(): String = proseTextAnnotatedProvider()

  override fun proseToSelectionAnnotated(start: Int, end: Int): Selection? = null

  companion object {
    private val AllStateFields =
      listOf(
        StateField.PageSizes,
        StateField.Selection,
        StateField.Cursor,
        StateField.Ime,
        StateField.Placeholder,
        StateField.TrackedRanges,
        StateField.TableOverlays,
        StateField.ExternalElements,
        StateField.RootAttrs,
        StateField.Modifiers,
        StateField.Block,
        StateField.LastHistoryTag,
      )

    val CoverAllRect = co.typie.editor.ffi.Rect(x = -1e6f, y = -1e6f, width = 2e6f, height = 2e6f)

    fun coveringHitRects(vararg pages: Int): List<PageRect> = pages.map {
      PageRect(pageIdx = it, rect = CoverAllRect)
    }

    fun coveringRegion(hit: InteractiveHit, page: Int = 0): InteractiveRegion =
      InteractiveRegion(
        pageIdx = page,
        entryRect = CoverAllRect,
        effectiveRect = CoverAllRect,
        hit = hit,
      )

    private val EmptyPosition = Position(node = "", offset = 0, affinity = Affinity.Downstream)
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
