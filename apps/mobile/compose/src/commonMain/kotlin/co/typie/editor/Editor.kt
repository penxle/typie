package co.typie.editor

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import co.touchlab.kermit.Logger
import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.CharacterCounts
import co.typie.editor.ffi.ClipboardPayload
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.ProseRangeInstallOutcome
import co.typie.editor.ffi.ProseTrackedRangeRegistration
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.SearchOptions
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.ThemeVariant
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.Viewport
import co.typie.editor.input.EditorInputRecorder
import co.typie.editor.sync.MissingBytes
import co.typie.editor.sync.PartitionedBytes
import co.typie.editor.sync.SplitChangeset
import co.typie.editor.sync.encodeLengthPrefixedBlobs
import co.typie.editor.sync.toChangesetBytes
import co.typie.platform.PlatformModule
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.coroutines.CoroutineContext
import kotlin.reflect.KClass
import kotlinx.collections.immutable.PersistentList
import kotlinx.collections.immutable.PersistentMap
import kotlinx.collections.immutable.PersistentSet
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.persistentSetOf
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext

// Window (flat chars each side of the selection) materialized into snapshot IME
// state; consumers resolve absolute offsets through Ime.windowStart, so a bounded
// window only limits how much surrounding text the IME can observe.
private const val IME_SNAPSHOT_WINDOW = 4096

private data class EditorAwaitTick<T>(
  val events: List<EditorEvent>,
  val snapshot: EditorState,
  val value: T,
)

@OptIn(ExperimentalAtomicApi::class)
class Editor
internal constructor(
  private val inner: co.typie.editor.ffi.Editor,
  val scope: CoroutineScope,
  private val dispatcher: CoroutineDispatcher = Dispatchers.Default.limitedParallelism(1),
  private val onError: (Editor, Throwable) -> Unit = { _, _ -> },
) {
  var state: EditorState by mutableStateOf(EditorState.Initial)
    private set

  // `state` lags render settlement on the await path; IME and pointer readers must see
  // post-tick values like the live FFI reads they replaced, so every readSnapshot is
  // published here immediately.
  private var tickSnapshot: EditorState by mutableStateOf(EditorState.Initial)

  internal var inputRecorder: EditorInputRecorder? = null

  // Selection-handle drags pause per-tick IME notifications: each one ships the
  // whole ime window (O(selection)) over the Android binder. The resume emission
  // delivers the final state once.
  internal var imeNotificationsPaused: Boolean by mutableStateOf(false)

  val cursor: CursorMetrics? by derivedStateOf { state.cursor }
  val placeholder: PlaceholderMetrics? by derivedStateOf { state.placeholder }
  val selection: Selection? by derivedStateOf { state.selection }
  val tickIme: Ime? by derivedStateOf { tickSnapshot.ime }
  val tickSelectionEndpoints: SelectionEndpoints? by derivedStateOf {
    tickSnapshot.selectionEndpoints
  }
  val pageSizes: List<Size> by derivedStateOf { state.pageSizes }
  val externalElements: List<ExternalElement> by derivedStateOf { state.externalElements }
  val tableOverlays: List<TableOverlay> by derivedStateOf { state.tableOverlays }
  val rootAttrs: PlainRootNode? by derivedStateOf { state.rootAttrs }
  val rootModifiers: List<EditorModifier>? by derivedStateOf { state.rootModifiers }
  val modifierState: ModifierState? by derivedStateOf { state.modifierState }
  val blockState: BlockState? by derivedStateOf { state.blockState }
  val ime: Ime? by derivedStateOf { state.ime }
  val lastHistoryTag by derivedStateOf { state.lastHistoryTag }

  private val mutex = PriorityMutex()
  private val versionCounter: AtomicLong = AtomicLong(0L)
  private val disposed: AtomicBoolean = AtomicBoolean(false)
  private val imeSessionActive: AtomicBoolean = AtomicBoolean(false)
  private val syncInProgress: AtomicBoolean = AtomicBoolean(false)
  private val localEdits = EditorLocalEditCoordinator()
  private val attachedPages: AtomicReference<PersistentSet<Int>> =
    AtomicReference(persistentSetOf())
  private val pendingSettles: AtomicReference<PersistentList<PendingSettle>> =
    AtomicReference(persistentListOf())
  private val queuedLocalEdits: AtomicReference<PersistentList<LocalEdit>> =
    AtomicReference(persistentListOf())
  private val queued: AtomicBoolean = AtomicBoolean(false)
  private val surfaceScheduler =
    EditorSurfaceScheduler(
      inner = inner,
      scope = scope,
      dispatcher = dispatcher,
      versionCounter = versionCounter,
      disposed = disposed,
      markPageAttached = { page -> attachedPages.updatePersistent { it.adding(page) } },
      markPageDetached = { page -> markSurfacePageDetached(page) },
      onPageSettled = { page, version -> onPageSettled(page, version) },
      notifyFailure = { error -> notifyFailure(error) },
    )

  @PublishedApi
  internal val listeners:
    AtomicReference<
      PersistentMap<KClass<out EditorEvent>, PersistentSet<(Editor, EditorEvent) -> Unit>>
    > =
    AtomicReference(persistentMapOf())

  internal val focusRequester: FocusRequester = FocusRequester()
  internal var focusManager: FocusManager? = null

  fun focus(): Boolean {
    if (disposed.load()) {
      return false
    }

    return focusRequester.requestFocus()
  }

  fun blur() {
    focusManager?.clearFocus()
  }

  fun deactivateScene() {
    focusManager?.clearFocus()
  }

  internal fun <T : Job> trackLocalEdit(start: (CoroutineContext) -> T): T? {
    val localEdit = localEdits.register() ?: return null
    val completion =
      try {
        start(localEdit)
      } catch (e: CancellationException) {
        localEdit.fail(e)
        throw e
      } catch (e: Throwable) {
        localEdit.fail(e)
        notifyFailure(e)
        return null
      }
    completion.invokeOnCompletion { failure ->
      if (failure == null) {
        localEdit.complete()
      } else {
        localEdit.fail(failure)
      }
    }
    return completion
  }

  internal fun quiesceLocalEdits(): LocalEditQuiescence = localEdits.quiesce()

  suspend fun await(
    beforeCommit: ((EditorState) -> Unit)? = null,
    admit: () -> Boolean = { true },
    block: EditorScope.() -> Unit,
  ): Boolean =
    await(beforeCommit = beforeCommit, admit = admit, mapEvents = { Unit }, block = block) != null

  internal suspend fun <T : Any> await(
    beforeCommit: ((EditorState) -> Unit)? = null,
    admit: () -> Boolean = { true },
    mapEvents: (List<EditorEvent>) -> T,
    block: EditorScope.() -> Unit,
  ): T? {
    try {
      val messages = mutableListOf<Message>()
      val collector =
        object : EditorScope {
          override fun enqueue(message: Message) {
            messages += message
          }
        }
      block(collector)
      if (messages.isEmpty()) return mapEvents(emptyList())

      return localEdits.run {
        val tick: EditorAwaitTick<T>? =
          withContext(NonCancellable + dispatcher) {
            mutex.withLock {
              if (disposed.load()) throw CancellationException("Editor disposed")
              if (!admit()) return@withLock null
              for (m in messages) inner.enqueue(m)
              val e = inner.tick()
              val version = versionCounter.addAndFetch(1L)
              val s = readSnapshot(version = version, events = e)
              EditorAwaitTick(events = e, snapshot = s, value = mapEvents(e))
            }
          }
        if (tick == null) return@run null
        val (events, snapshot, value) = tick

        val pending: PendingSettle? =
          if (events.any { it is EditorEvent.RenderInvalidated }) {
            val initial = attachedPages.load()
            if (initial.isEmpty()) null
            else
              PendingSettle(initial, requiredVersion = snapshot.version).also {
                pendingSettles.updatePersistent { list -> list.add(it) }
              }
          } else null

        try {
          emit(events)
          pending?.await()
        } finally {
          if (pending != null) {
            pendingSettles.updatePersistent { it.remove(pending) }
          }
          withContext(NonCancellable) {
            mutex.withLock {
              if (!disposed.load()) {
                beforeCommit?.invoke(snapshot)
                commit(snapshot)
              }
            }
          }
        }
        value
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      notifyFailure(e)
      throw e
    }
  }

  fun sync(beforeCommit: ((EditorState) -> Unit)? = null, block: EditorScope.() -> Unit) {
    val localEdit = localEdits.register() ?: return
    if (!syncInProgress.compareAndSet(expectedValue = false, newValue = true)) {
      val error = IllegalStateException("nested sync is not supported")
      localEdit.fail(error)
      notifyFailure(error)
      return
    }
    try {
      runBlocking {
        val events = mutex.withPriorityLock {
          if (disposed.load()) error("Editor disposed")
          val collector =
            object : EditorScope {
              override fun enqueue(message: Message) {
                inner.enqueue(message)
              }
            }
          block(collector)
          val e = inner.tick()
          val version = versionCounter.addAndFetch(1L)
          val snapshot = readSnapshot(version = version, events = e)
          beforeCommit?.invoke(snapshot)
          commit(snapshot)
          e
        }
        emit(events)
      }
      localEdit.complete()
    } catch (e: Throwable) {
      localEdit.fail(e)
      notifyFailure(e)
    } finally {
      syncInProgress.store(false)
    }
  }

  // Materializing the snapshot ime window is O(selection + context) per call, so
  // it only happens while a platform text input session is attached; activation
  // must be followed by refreshImeSnapshot so the session never reads a stale
  // (or missing) window.
  internal fun setImeSessionActive(active: Boolean) {
    imeSessionActive.store(active)
  }

  // Composition teardown and ime gating must decide together under the editor
  // lock: clearing the flag first would let a concurrent tick publish a null
  // ime and hide a live composition from the teardown check. CommitAsIs stays
  // conditional — the core runs text replacement on every commit dispatch, so
  // an unconditional dispatch would fire it on plain blurs.
  internal fun deactivateImeSession() {
    if (!imeSessionActive.load()) return
    sync {
      if (tickIme?.composing != null) {
        enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
      }
      setImeSessionActive(false)
    }
  }

  // A versioned tick + commit on purpose: an out-of-band `state.copy(ime = …)`
  // patch would be wiped back to null when a settle-parked pre-activation
  // snapshot later lands, starving pull-based platforms of the ime window. The
  // committed snapshot outranks the parked one instead, and every carried field
  // in readSnapshot derives from tickSnapshot, so the parked edit's values
  // (documentRevision included) survive in it.
  internal suspend fun refreshImeSnapshot() {
    withSuspendFailureNotification {
      val events =
        withContext(NonCancellable + dispatcher) {
          mutex.withLock {
            if (disposed.load() || !imeSessionActive.load()) return@withLock emptyList()
            // Current means the committed state agrees with the latest tick — a
            // non-null tickSnapshot.ime alone can belong to a settle-parked
            // snapshot whose commit hasn't landed, and pull-based platforms read
            // state.ime.
            if (tickSnapshot.ime != null && state.ime == tickSnapshot.ime) {
              return@withLock emptyList()
            }
            val e = inner.tick()
            val version = versionCounter.addAndFetch(1L)
            commit(readSnapshot(version = version, events = e))
            e
          }
        }
      emit(events)
    }
  }

  fun enqueue(message: Message) {
    if (disposed.load()) return
    val localEdit = localEdits.register() ?: return
    try {
      inner.enqueue(message)
      queuedLocalEdits.updatePersistent { it.adding(localEdit) }
      scheduleTick()
    } catch (e: Throwable) {
      localEdit.fail(e)
      notifyFailure(e)
      throw e
    }
  }

  suspend fun freezeSelection(selection: Selection): StableSelection? =
    readInner(defaultValue = { null }) { it.freezeSelection(selection) }

  suspend fun findMatches(query: String, matchWholeWord: Boolean): List<Selection> =
    readInner(defaultValue = { emptyList() }) {
      it.findMatches(query, SearchOptions(matchWholeWord = matchWholeWord))
    }

  suspend fun proseText(): String = readInner(defaultValue = { "" }) { it.proseText() }

  suspend fun proseToSelection(start: Int, end: Int): Selection? =
    readInner(defaultValue = { null }) { it.proseToSelection(start, end) }

  suspend fun proseTextAnnotated(): String =
    readInner(defaultValue = { "" }) { it.proseTextAnnotated() }

  suspend fun proseToSelectionAnnotated(start: Int, end: Int): Selection? =
    readInner(defaultValue = { null }) { it.proseToSelectionAnnotated(start, end) }

  internal suspend fun replaceTrackedRangeGroupsFromProse(
    expectedText: String,
    groups: List<String>,
    ranges: List<ProseTrackedRangeRegistration>,
    isCurrent: () -> Boolean,
  ): ProseRangeInstallOutcome? {
    val outcome =
      await(
        admit = isCurrent,
        mapEvents = { events ->
          val matches = events.filterIsInstance<EditorEvent.ProseRangeInstallResult>()
          check(matches.size == 1) {
            "Expected exactly one prose range install result, got ${matches.size}"
          }
          matches.single().outcome
        },
      ) {
        enqueue(
          Message.TrackedRange(
            TrackedRangeOp.ReplaceGroupsFromProse(
              expectedText = expectedText,
              groups = groups,
              ranges = ranges,
            )
          )
        )
      }
    return outcome?.takeIf { isCurrent() }
  }

  internal suspend fun clearTrackedRangeGroups(
    groups: List<String>,
    admit: () -> Boolean = { true },
  ): Boolean =
    await(admit = admit, mapEvents = { Unit }) {
      groups.forEach { group ->
        enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = group)))
      }
    } != null

  private fun scheduleTick() {
    if (!queued.compareAndSet(expectedValue = false, newValue = true)) return
    scope.launch(dispatcher) {
      var edits: PersistentList<LocalEdit> = persistentListOf()
      try {
        val events =
          withContext(dispatcher) {
            mutex.withLock {
              queued.store(false)
              edits = queuedLocalEdits.exchange(persistentListOf())
              if (disposed.load()) return@withLock emptyList()
              val e = inner.tick()
              val version = versionCounter.addAndFetch(1L)
              commit(readSnapshot(version = version, events = e))
              e
            }
          }
        emit(events)
        edits.forEach(LocalEdit::complete)
      } catch (e: CancellationException) {
        edits.forEach { localEdit -> localEdit.fail(e) }
        throw e
      } catch (e: Throwable) {
        edits.forEach { localEdit -> localEdit.fail(e) }
        notifyFailure(e)
      }
    }
  }

  inline fun <reified T : EditorEvent> on(noinline listener: EditorEventListener<T>): () -> Unit {
    @Suppress("UNCHECKED_CAST") val wrapped = listener as (Editor, EditorEvent) -> Unit
    val key = T::class
    listeners.updatePersistent { map ->
      val set = map[key] ?: persistentSetOf()
      map.put(key, set.add(wrapped))
    }
    return {
      listeners.updatePersistent { map ->
        val set = map[key] ?: return@updatePersistent map
        map.put(key, set.remove(wrapped))
      }
    }
  }

  @PublishedApi
  internal inline fun <T> AtomicReference<T>.updatePersistent(transform: (T) -> T): T {
    while (true) {
      val current = load()
      val next = transform(current)
      if (compareAndSet(current, next)) return next
    }
  }

  private fun emit(events: List<EditorEvent>) {
    if (events.isEmpty()) return
    val filtered = events.filterNot { it is EditorEvent.StateChanged }
    if (filtered.isEmpty()) return
    scope.launch(Dispatchers.Main) {
      for (event in filtered) {
        val snapshot = listeners.load()[event::class] ?: continue
        for (listener in snapshot) {
          try {
            listener(this@Editor, event)
          } catch (e: CancellationException) {
            throw e
          } catch (e: Throwable) {
            Logger.e(e) { "Editor listener threw for ${event::class.simpleName}" }
            notifyFailure(e)
          }
        }
      }
    }
  }

  internal fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
  ): SurfaceSessionHandle = surfaceScheduler.attachSurface(page, handle, width, height, scaleFactor)

  private fun markSurfacePageDetached(page: Int) {
    attachedPages.updatePersistent { it.removing(page) }
    pendingSettles.load().forEach { it.markDetached(page) }
  }

  fun onPageSettled(page: Int, version: Long) {
    pendingSettles.load().forEach { it.markCommitted(page, version) }
  }

  fun inspectState(options: InspectStateOptions? = null): String = inner.inspectState(options)

  fun inspectStateAsMacro(): String = inner.inspectStateAsMacro()

  // Snapshot-based on purpose: these run synchronously inside touch dispatch on the
  // main thread, where a direct FFI call would contend on the Rust editor mutex and
  // stall input for as long as a background tick holds it (APP2-7P).
  fun selectionHitTest(page: Int, x: Float, y: Float): Boolean =
    tickSnapshot.selectionHitRects.any { it.pageIdx == page && it.rect.containsPoint(x, y) }

  fun cursorHitTest(page: Int, x: Float, y: Float): Boolean =
    tickSnapshot.cursorHitRects.any { it.pageIdx == page && it.rect.containsPoint(x, y) }

  fun interactiveHitTest(page: Int, x: Float, y: Float): InteractiveHit? {
    val region =
      tickSnapshot.interactiveRegions.firstOrNull {
        it.pageIdx == page && it.entryRect.containsPoint(x, y)
      } ?: return null
    return if (region.effectiveRect.containsPoint(x, y)) region.hit else null
  }

  private fun Rect.containsPoint(px: Float, py: Float): Boolean =
    px >= x && px <= x + width && py >= y && py <= y + height

  suspend fun characterCounts(): CharacterCounts? =
    withSuspendFailureNotification(defaultValue = { null }) {
      withContext(dispatcher) {
        mutex.withLock {
          if (disposed.load()) {
            null
          } else {
            inner.characterCounts()
          }
        }
      }
    }

  suspend fun copySelection(): ClipboardPayload? =
    withSuspendFailureNotification(defaultValue = { null }) {
      withContext(dispatcher) {
        mutex.withLock {
          if (disposed.load()) {
            null
          } else {
            inner.copySelection()
          }
        }
      }
    }

  internal suspend fun collectLocalChangesets(
    baseHeads: ByteArray?,
    block: EditorScope.() -> Unit,
  ): EditorLocalChangesets {
    val resolvedBaseHeads = baseHeads ?: currentHeads()
    await(block = block)
    val changesets = localChangesetsSince(resolvedBaseHeads)
    val currentHeads = currentHeads()
    return EditorLocalChangesets(
      baseHeads = resolvedBaseHeads,
      currentHeads = currentHeads,
      changesets = changesets,
    )
  }

  internal suspend fun changesetIds(): List<String> =
    withContext(dispatcher) {
      mutex.withLock {
        if (disposed.load()) throw CancellationException("Editor disposed")
        inner.changesetIds()
      }
    }

  internal suspend fun missingChangesetsFor(remoteHeads: ByteArray): MissingBytes =
    withContext(dispatcher) {
      mutex.withLock {
        if (disposed.load()) throw CancellationException("Editor disposed")
        val result = inner.missingChangesetsTolerant(remoteHeads)
        MissingBytes(bytes = result.bytes.toChangesetBytes(), withheld = result.withheld)
      }
    }

  internal suspend fun splitChangesets(payload: ByteArray): List<SplitChangeset> =
    withContext(dispatcher) {
      mutex.withLock {
        if (disposed.load()) throw CancellationException("Editor disposed")
        inner.splitChangesets(payload).map {
          SplitChangeset(id = it.id, bytes = it.bytes.toChangesetBytes())
        }
      }
    }

  internal suspend fun partitionRemoteChangesets(payload: ByteArray): PartitionedBytes =
    withContext(dispatcher) {
      mutex.withLock {
        if (disposed.load()) throw CancellationException("Editor disposed")
        val partitioned = inner.partitionRemoteChangesets(payload)
        PartitionedBytes(
          ready = partitioned.ready.toChangesetBytes(),
          blocked = partitioned.blocked.toChangesetBytes(),
        )
      }
    }

  internal suspend fun receiveRemoteChangeset(payload: ByteArray) {
    withSuspendFailureNotification {
      val events =
        withContext(NonCancellable + dispatcher) {
          mutex.withLock {
            if (disposed.load()) throw CancellationException("Editor disposed")
            inner.receiveRemoteChangeset(payload)
            val e = inner.tick()
            val version = versionCounter.addAndFetch(1L)
            commit(readSnapshot(version = version, events = e))
            e
          }
        }
      emit(events)
    }
  }

  suspend fun insertTemplateFragment(changesets: ByteArray): Boolean =
    try {
      localEdits.run {
        val events =
          withContext(NonCancellable + dispatcher) {
            mutex.withLock {
              if (disposed.load()) throw CancellationException("Editor disposed")
              inner.insertTemplateFragment(changesets)
              val e = inner.tick()
              val version = versionCounter.addAndFetch(1L)
              commit(readSnapshot(version = version, events = e))
              e
            }
          }
        emit(events)
        true
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      notifyFailure(e)
      false
    }

  internal suspend fun currentHeads(): ByteArray =
    withContext(dispatcher) {
      mutex.withLock {
        if (disposed.load()) throw CancellationException("Editor disposed")
        inner.currentHeads()
      }
    }

  private suspend fun localChangesetsSince(remoteHeads: ByteArray): ByteArray =
    withContext(dispatcher) {
      mutex.withLock {
        if (disposed.load()) throw CancellationException("Editor disposed")
        inner.localChangesetsSince(remoteHeads)
      }
    }

  fun dispose() {
    if (!disposed.compareAndSet(expectedValue = false, newValue = true)) return
    EditorRegistry.unregisterAsync(this)
    pendingSettles.exchange(persistentListOf()).forEach { it.cancel() }
    queuedLocalEdits.exchange(persistentListOf()).forEach(LocalEdit::complete)
    listeners.store(persistentMapOf())
    attachedPages.store(persistentSetOf())
    surfaceScheduler.dispose()
  }

  private fun readSnapshot(version: Long, events: List<EditorEvent>): EditorState {
    val selection = inner.selection()
    val selectionEndpoints =
      if (selection != null && selection.anchor != selection.head) {
        inner.selectionEndpoints()
      } else {
        null
      }
    val documentChanged = events.hasStateChangedField(StateField.Doc)
    val placeholderChanged = events.hasStateChangedField(StateField.Placeholder)
    val trackedRangesChanged = events.hasStateChangedField(StateField.TrackedRanges)
    val tableOverlaysChanged = events.hasStateChangedField(StateField.TableOverlays)
    val lastHistoryTagChanged = events.hasStateChangedField(StateField.LastHistoryTag)
    val renderInvalidated = events.any { it is EditorEvent.RenderInvalidated }
    // Hit rects carry over from tickSnapshot, not state: state lags render settlement,
    // and a settle-delayed commit would resurrect pre-refresh geometry.
    val hitGeometryStale =
      tickSnapshot.selection != selection ||
        documentChanged ||
        renderInvalidated ||
        tickSnapshot.version == 0L
    val selectionHitRects =
      when {
        selection == null || selection.anchor == selection.head -> emptyList()
        hitGeometryStale -> inner.selectionHitRects()
        else -> tickSnapshot.selectionHitRects
      }
    val cursorHitRects =
      when {
        selection == null || selection.anchor != selection.head -> emptyList()
        hitGeometryStale -> inner.cursorHitRects()
        else -> tickSnapshot.cursorHitRects
      }
    val interactiveRegions =
      if (documentChanged || renderInvalidated || tickSnapshot.version == 0L) {
        inner.interactiveRegions()
      } else {
        tickSnapshot.interactiveRegions
      }
    val imeChanged = events.hasStateChangedField(StateField.Ime)
    val ime =
      when {
        !imeSessionActive.load() -> null
        imeChanged || tickSnapshot.ime == null ->
          inner.ime(IME_SNAPSHOT_WINDOW, IME_SNAPSHOT_WINDOW)
        else -> tickSnapshot.ime
      }
    // Every carry-over below reads tickSnapshot, not state, for the same reason
    // as the hit rects above: state lags render settlement, so a tick that
    // interleaves with a settle-parked commit would otherwise fork from stale
    // values — and once its higher version lands, the version guard drops the
    // parked commit and its field updates (documentRevision included) for good.
    val placeholder =
      if (placeholderChanged || tickSnapshot.version == 0L) {
        inner.placeholder()
      } else {
        tickSnapshot.placeholder
      }
    val trackedRanges =
      if (trackedRangesChanged) {
        inner.trackedRanges(null)
      } else {
        tickSnapshot.trackedRanges
      }
    val tableOverlays =
      if (tableOverlaysChanged || tickSnapshot.version == 0L) {
        inner.tableOverlays()
      } else {
        tickSnapshot.tableOverlays
      }
    val selectionChanged = tickSnapshot.selection != selection
    val snapshot =
      EditorState(
        version = version,
        documentRevision = tickSnapshot.documentRevision + if (documentChanged) 1L else 0L,
        cursor = inner.cursor(),
        placeholder = placeholder,
        selection = selection,
        selectionEndpoints = selectionEndpoints,
        pageSizes = inner.pageSizes(),
        externalElements = inner.externalElements(),
        tableOverlays = tableOverlays,
        rootAttrs = inner.rootAttrs(),
        rootModifiers = inner.rootModifiers(),
        modifierState = inner.modifierState(),
        blockState = inner.blockState(),
        ime = ime,
        lastHistoryTag =
          if (lastHistoryTagChanged || tickSnapshot.version == 0L) {
            inner.lastHistoryTag()
          } else {
            tickSnapshot.lastHistoryTag
          },
        trackedRanges = trackedRanges,
        trackedRangesContainingSelectionHead =
          if (selection != null && selection.anchor == selection.head) {
            if (selectionChanged || trackedRangesChanged) {
              inner.trackedRangesContainingPosition(selection.head, null)
            } else {
              tickSnapshot.trackedRangesContainingSelectionHead
            }
          } else {
            emptyList()
          },
        selectionHitRects = selectionHitRects,
        cursorHitRects = cursorHitRects,
        interactiveRegions = interactiveRegions,
      )
    tickSnapshot = snapshot
    return snapshot
  }

  private fun List<EditorEvent>.hasStateChangedField(field: StateField): Boolean = any { event ->
    event is EditorEvent.StateChanged && field in event.fields
  }

  private fun notifyFailure(error: Throwable) {
    if (error is CancellationException) {
      throw error
    }
    onError(this, error)
  }

  private fun withFailureNotification(block: () -> Unit) {
    try {
      block()
    } catch (e: Throwable) {
      notifyFailure(e)
    }
  }

  private fun <T> withFailureNotification(defaultValue: () -> T, block: () -> T): T =
    try {
      block()
    } catch (e: Throwable) {
      notifyFailure(e)
      defaultValue()
    }

  private suspend fun <T> readInner(
    defaultValue: () -> T,
    block: (co.typie.editor.ffi.Editor) -> T,
  ): T =
    withSuspendFailureNotification(defaultValue) {
      withContext(dispatcher) {
        mutex.withLock {
          if (disposed.load()) error("Editor disposed")
          block(inner)
        }
      }
    }

  private suspend fun withSuspendFailureNotification(block: suspend () -> Unit) {
    try {
      block()
    } catch (e: Throwable) {
      notifyFailure(e)
    }
  }

  private suspend fun <T> withSuspendFailureNotification(
    defaultValue: () -> T,
    block: suspend () -> T,
  ): T =
    try {
      block()
    } catch (e: Throwable) {
      notifyFailure(e)
      defaultValue()
    }

  private fun commit(snapshot: EditorState) {
    if (snapshot.version <= state.version) return
    state = snapshot
  }

  companion object {
    suspend fun create(
      graph: ByteArray,
      viewport: Viewport,
      scope: CoroutineScope,
      themeVariant: ThemeVariant = ThemeVariant.LightWhite,
      dispatcher: CoroutineDispatcher = Dispatchers.Default.limitedParallelism(1),
      onError: (Editor, Throwable) -> Unit = { _, _ -> },
    ): Editor =
      createInitialized(
        scope = scope,
        themeVariant = themeVariant,
        dispatcher = dispatcher,
        onError = onError,
        createInner = { PlatformModule.editorHost.createEditorFromGraph(graph, viewport) },
      )

    suspend fun createWithPending(
      graph: ByteArray,
      pending: List<ByteArray>,
      viewport: Viewport,
      scope: CoroutineScope,
      themeVariant: ThemeVariant = ThemeVariant.LightWhite,
      dispatcher: CoroutineDispatcher = Dispatchers.Default.limitedParallelism(1),
      onError: (Editor, Throwable) -> Unit = { _, _ -> },
    ): Editor =
      createInitialized(
        scope = scope,
        themeVariant = themeVariant,
        dispatcher = dispatcher,
        onError = onError,
        createInner = {
          PlatformModule.editorHost.createEditorFromGraphWithPending(
            graph,
            encodeLengthPrefixedBlobs(pending),
            viewport,
          )
        },
      )

    suspend fun createFromDoc(
      doc: PlainDoc,
      viewport: Viewport,
      scope: CoroutineScope,
      themeVariant: ThemeVariant = ThemeVariant.LightWhite,
      dispatcher: CoroutineDispatcher = Dispatchers.Default.limitedParallelism(1),
      onError: (Editor, Throwable) -> Unit = { _, _ -> },
    ): Editor =
      createInitialized(
        scope = scope,
        themeVariant = themeVariant,
        dispatcher = dispatcher,
        onError = onError,
        createInner = { PlatformModule.editorHost.createEditorFromDoc(doc, viewport) },
      )

    internal suspend fun createInitialized(
      scope: CoroutineScope,
      themeVariant: ThemeVariant,
      dispatcher: CoroutineDispatcher,
      onError: (Editor, Throwable) -> Unit,
      createInner: () -> co.typie.editor.ffi.Editor,
    ): Editor {
      var createdEditor: Editor? = null
      return try {
        val editor =
          withContext(Dispatchers.Default) {
            val editor = Editor(createInner(), scope, dispatcher, onError)
            createdEditor = editor

            editor.on<EditorEvent.FontDataMissing>(FontLoader.fontDataMissingHandler)
            var initialized = false
            EditorRegistry.register(editor)
            try {
              PlatformModule.editorHost.setThemeVariant(themeVariant)
              editor.await(mapEvents = { Unit }) {
                enqueue(Message.System(SystemEvent.ThemeVariantChanged))
                enqueue(Message.System(SystemEvent.Initialize))
              }
              initialized = true
            } finally {
              if (!initialized) {
                EditorRegistry.unregister(editor)
              }
            }

            editor
          }
        createdEditor = null
        editor
      } finally {
        createdEditor?.dispose()
      }
    }
  }
}
