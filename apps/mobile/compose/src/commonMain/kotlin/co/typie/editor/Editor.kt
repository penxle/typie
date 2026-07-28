package co.typie.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.IntSize
import co.touchlab.kermit.Logger
import co.typie.editor.ffi.CharacterCounts
import co.typie.editor.ffi.ClipboardPayload
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.ProseRangeInstallOutcome
import co.typie.editor.ffi.ProseTrackedRangeRegistration
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.RequestId
import co.typie.editor.ffi.ResourceUpdate
import co.typie.editor.ffi.SearchOptions
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.ThemeVariant
import co.typie.editor.ffi.TickResult
import co.typie.editor.ffi.TrackedRange
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
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext

// Window (flat chars each side of the selection) materialized into snapshot IME
// state; consumers resolve absolute offsets through Ime.windowStart, so a bounded
// window only limits how much surrounding text the IME can observe.
private const val IME_SNAPSHOT_WINDOW = 4096

private data class PublicationWaiter(
  val revision: Long,
  val completion: CompletableDeferred<EditorPublicationResult>,
)

internal data class PublishedBundle(val snapshot: EditorState, val frames: Map<Int, PresentedFrame>)

private class SurfacePage(
  var target: SurfaceTarget,
  var session: SurfaceSessionHandle,
  var wakeDelivery: (FrameKey) -> Unit,
  var requiredRevision: Long?,
  var frame: PresentedFrame? = null,
  var available: Boolean = true,
  var inFlightRevision: Long? = null,
  var inFlightSurfaceKey: SurfaceKey? = null,
  var preparedFrameKey: FrameKey? = null,
  var failedRevision: Long? = null,
)

private class VisualHost(val token: Any, val pages: MutableMap<Int, SurfacePage> = mutableMapOf())

@OptIn(ExperimentalAtomicApi::class)
class Editor
internal constructor(
  private val inner: co.typie.editor.ffi.Editor,
  val scope: CoroutineScope,
  private val dispatcher: CoroutineDispatcher = Dispatchers.Default.limitedParallelism(1),
  private val onError: (Editor, Throwable) -> Unit = { _, _ -> },
) {
  // Logical consumers use the latest applied snapshot. Visual consumers use
  // publishedState, which advances only with matching delivered surface proofs.
  var appliedState: EditorState by mutableStateOf(EditorState.Initial)
    private set

  val publishedState: EditorState
    get() = published?.snapshot ?: EditorState.Initial

  internal val publishedBundle: PublishedBundle?
    get() = published

  val appliedRevision: Long
    get() = appliedState.version

  val publishedRevision: Long?
    get() = published?.snapshot?.version

  internal var inputRecorder: EditorInputRecorder? = null

  // Selection-handle drags pause per-tick IME notifications: each one ships the
  // whole ime window (O(selection)) over the Android binder. The resume emission
  // delivers the final state once.
  internal var imeNotificationsPaused: Boolean by mutableStateOf(false)

  val terminal: Boolean
    get() = disposed.load() || failed.load()

  private val mutex = PriorityMutex()
  private val disposed: AtomicBoolean = AtomicBoolean(false)
  private val failed: AtomicBoolean = AtomicBoolean(false)
  private val imeSessionActive: AtomicBoolean = AtomicBoolean(false)
  private val syncInProgress: AtomicBoolean = AtomicBoolean(false)
  private val localEdits = EditorLocalEditCoordinator()
  private val nextSurfaceKey = AtomicLong(0L)
  private var visualHost: VisualHost? = null
  private var published: PublishedBundle? by mutableStateOf(null)
  // Native buffer readers cannot wait for a tick that owns the Editor mutex.
  // Writers refresh this immutable derived view while holding that mutex.
  private val retainedFrameBitmaps:
    AtomicReference<PersistentMap<Int, PersistentList<ImageBitmap>>> =
    AtomicReference(persistentMapOf())
  private val queuedLocalEdits: AtomicReference<PersistentList<LocalEdit>> =
    AtomicReference(persistentListOf())
  private val requestReceipts:
    AtomicReference<PersistentMap<Long, CompletableDeferred<EditorUpdate>>> =
    AtomicReference(persistentMapOf())
  private val publicationWaiters: AtomicReference<PersistentList<PublicationWaiter>> =
    AtomicReference(persistentListOf())
  private val queued: AtomicBoolean = AtomicBoolean(false)
  private val surfaceDriver =
    SurfaceDriver(
      inner = inner,
      scope = scope,
      dispatcher = dispatcher,
      disposed = disposed,
      failed = failed,
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
    if (terminal) {
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
    if (terminal) return null

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

  suspend fun update(
    admit: () -> Boolean = { true },
    block: EditorRequestScope.() -> Unit,
  ): EditorUpdate? = updateInternal(admit = admit, afterApplied = null, block = block)

  internal suspend fun update(
    admit: () -> Boolean = { true },
    afterApplied: (EditorUpdate) -> Unit,
    block: EditorRequestScope.() -> Unit,
  ): EditorUpdate? = updateInternal(admit = admit, afterApplied = afterApplied, block = block)

  private suspend fun updateInternal(
    admit: () -> Boolean,
    afterApplied: ((EditorUpdate) -> Unit)?,
    block: EditorRequestScope.() -> Unit,
  ): EditorUpdate? {
    if (terminal) return null

    val messages = collectMessages(block)
    if (messages.isEmpty()) return null

    val receipt = CompletableDeferred<EditorUpdate>()
    val inheritedLocalEdit =
      currentCoroutineContext()[LocalEdit]?.takeIf { it.belongsTo(localEdits) }
    val localEdit = inheritedLocalEdit ?: localEdits.register() ?: return null
    val ownsLocalEdit = inheritedLocalEdit == null
    var admitted = false
    var afterAppliedCompletion: CompletableDeferred<Unit>? = null
    val update =
      try {
        val accepted =
          withContext(dispatcher) {
            mutex.withLock {
              ensureActive()
              if (!admit()) return@withLock false
              val requestId = inner.enqueueRequest(messages)
              admitted = true
              requestReceipts.updatePersistent { it.putting(requestId.value, receipt) }
              if (ownsLocalEdit) {
                receipt.invokeOnCompletion { error ->
                  if (error == null) localEdit.complete() else localEdit.fail(error)
                }
              }
              if (afterApplied != null) {
                val completion = CompletableDeferred<Unit>()
                afterAppliedCompletion = completion
                scope.launch(start = CoroutineStart.UNDISPATCHED) {
                  try {
                    afterApplied(receipt.await())
                    completion.complete(Unit)
                  } catch (error: Throwable) {
                    completion.completeExceptionally(error)
                  }
                }
              }
              scheduleTick()
              true
            }
          }
        if (!accepted) {
          if (ownsLocalEdit) localEdit.complete()
          return null
        }
        receipt.await()
      } catch (e: CancellationException) {
        if (ownsLocalEdit && !admitted) localEdit.fail(e)
        throw e
      } catch (e: Throwable) {
        receipt.completeExceptionally(e)
        if (ownsLocalEdit && !admitted) localEdit.fail(e)
        fail(e)
        throw e
      }
    afterAppliedCompletion?.await()
    return update
  }

  fun updateNow(block: EditorRequestScope.() -> Unit): EditorUpdate? =
    updateNow(admit = { true }, block = block)

  internal fun updateNow(
    admit: () -> Boolean,
    block: EditorRequestScope.() -> Unit,
  ): EditorUpdate? {
    if (!syncInProgress.compareAndSet(expectedValue = false, newValue = true)) {
      error("nested updateNow is not supported")
    }
    return try {
      if (terminal) return null

      val messages = collectMessages(block)
      if (messages.isEmpty()) return null

      val localEdit = localEdits.register() ?: return null
      val result =
        try {
          runBlocking {
            mutex.withPriorityLock {
              ensureActive()
              if (!admit()) return@withPriorityLock null
              val requestId = inner.enqueueRequest(messages)
              val tick = inner.tickThrough(requestId)
              install(tick)
              updateFor(tick, requestId)
            }
          }
        } catch (e: CancellationException) {
          localEdit.fail(e)
          throw e
        } catch (e: Throwable) {
          localEdit.fail(e)
          fail(e)
          throw e
        }
      localEdit.complete()
      if (result != null) emit(result.events)
      result
    } finally {
      syncInProgress.store(false)
    }
  }

  private fun collectMessages(block: EditorRequestScope.() -> Unit): List<Message> = buildList {
    block(
      object : EditorRequestScope {
        override fun enqueue(message: Message) {
          add(message)
        }
      }
    )
  }

  // Materializing the snapshot ime window is O(selection + context) per call, so
  // it only happens while a platform text input session is attached; activation
  // must be followed by refreshImeSnapshot so the session never reads a stale
  // (or missing) window.
  internal fun setImeSessionActive(active: Boolean) {
    imeSessionActive.store(active)
  }

  // CommitAsIs stays conditional because the core runs text replacement on every
  // commit dispatch; an unconditional dispatch would fire it on plain blurs.
  internal fun deactivateImeSession() {
    if (!imeSessionActive.load()) return
    try {
      updateNow(admit = { appliedState.ime?.composing != null }) {
        enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
      }
    } catch (error: Throwable) {
      if (!terminal) throw error
    } finally {
      setImeSessionActive(false)
    }
  }

  // IME materialization is a Host read preference, not a core mutation. It keeps
  // the same applied revision instead of creating an empty barrier tick.
  internal suspend fun refreshImeSnapshot() {
    withSuspendFailureNotification {
      withContext(dispatcher) {
        mutex.withLock {
          if (terminal || !imeSessionActive.load()) return@withLock
          val ime = inner.ime(IME_SNAPSHOT_WINDOW, IME_SNAPSHOT_WINDOW)
          if (appliedState.ime != ime) {
            appliedState = appliedState.copy(ime = ime)
          }
        }
      }
    }
  }

  fun enqueue(message: Message) {
    if (terminal) return
    val localEdit = localEdits.register() ?: return
    try {
      inner.enqueueRequest(listOf(message))
      queuedLocalEdits.updatePersistent { it.adding(localEdit) }
      scheduleTick()
    } catch (e: Throwable) {
      localEdit.fail(e)
      fail(e)
      throw e
    }
  }

  internal fun receiveResourceUpdate(update: ResourceUpdate) {
    if (terminal) return
    try {
      inner.receiveResourceUpdate(update)
      scheduleTick()
    } catch (e: Throwable) {
      fail(e)
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

  suspend fun trackedRange(id: String): TrackedRange? =
    readInner(defaultValue = { null }) { it.trackedRange(id) }

  internal suspend fun replaceTrackedRangeGroupsFromProse(
    expectedText: String,
    groups: List<String>,
    ranges: List<ProseTrackedRangeRegistration>,
    isCurrent: () -> Boolean,
  ): ProseRangeInstallOutcome? {
    val update =
      update(admit = isCurrent) {
        enqueue(
          Message.TrackedRange(
            TrackedRangeOp.ReplaceGroupsFromProse(
              expectedText = expectedText,
              groups = groups,
              ranges = ranges,
            )
          )
        )
      } ?: return null
    val matches = update.events.filterIsInstance<EditorEvent.ProseRangeInstallResult>()
    check(matches.size == 1) {
      "Expected exactly one prose range install result, got ${matches.size}"
    }
    val outcome = matches.single().outcome
    return outcome.takeIf { isCurrent() }
  }

  internal suspend fun clearTrackedRangeGroups(
    groups: List<String>,
    admit: () -> Boolean = { true },
  ): Boolean =
    update(admit = admit) {
      groups.forEach { group ->
        enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = group)))
      }
    } != null

  private fun scheduleTick() {
    if (!queued.compareAndSet(expectedValue = false, newValue = true)) return
    scope.launch(dispatcher) {
      var edits: PersistentList<LocalEdit> = persistentListOf()
      try {
        val events = mutex.withLock {
          queued.store(false)
          edits = queuedLocalEdits.exchange(persistentListOf())
          if (terminal) return@withLock emptyList()
          val tick = inner.tick() ?: return@withLock emptyList()
          install(tick)
          publicEvents(tick.events)
        }
        emit(events)
        edits.forEach(LocalEdit::complete)
      } catch (e: CancellationException) {
        edits.forEach { localEdit -> localEdit.fail(e) }
        throw e
      } catch (e: Throwable) {
        edits.forEach { localEdit -> localEdit.fail(e) }
        fail(e)
      }
    }
  }

  inline fun <reified T : EditorEvent> on(noinline listener: EditorEventListener<T>): () -> Unit {
    @Suppress("UNCHECKED_CAST") val wrapped = listener as (Editor, EditorEvent) -> Unit
    val key = T::class
    listeners.updatePersistent { map ->
      val set = map[key] ?: persistentSetOf()
      map.putting(key, set.adding(wrapped))
    }
    return {
      listeners.updatePersistent { map ->
        val set = map[key] ?: return@updatePersistent map
        map.putting(key, set.removing(wrapped))
      }
    }
  }

  private fun updateFor(tick: TickResult, requestId: RequestId): EditorUpdate {
    val request =
      tick.requestOutcomes.firstOrNull { it.requestId == requestId }
        ?: error("tick did not include request ${requestId.value}")
    return EditorUpdate(
      revision = tick.revision.value,
      snapshot = appliedState,
      events = publicEvents(tick.events),
      commandOutcomes = request.commandOutcomes,
      editor = this,
    )
  }

  private fun install(tick: TickResult) {
    readSnapshot(version = tick.revision.value, events = tick.events)
    reconcileSurfaceTargets(
      revision = tick.revision.value,
      renderInvalidated = tick.events.any { it is EditorEvent.RenderInvalidated },
    )
    startSurfaceRenders()
    publishIfReady()

    val events = publicEvents(tick.events)
    for (request in tick.requestOutcomes) {
      val receipt = requestReceipts.load()[request.requestId.value] ?: continue
      requestReceipts.updatePersistent { it.removing(request.requestId.value) }
      receipt.complete(
        EditorUpdate(
          revision = tick.revision.value,
          snapshot = appliedState,
          events = events,
          commandOutcomes = request.commandOutcomes,
          editor = this,
        )
      )
    }
  }

  /**
   * Keeps native targets on the same applied page geometry that will be published. SurfaceDriver
   * executes the resize/detach effects, but does not duplicate these target, requirement, or proof
   * facts.
   */
  private fun reconcileSurfaceTargets(revision: Long, renderInvalidated: Boolean) {
    val host = visualHost ?: return
    val pages = host.pages.iterator()
    while (pages.hasNext()) {
      val (_, page) = pages.next()
      val size = appliedState.pageSizes.getOrNull(page.target.page)
      if (size == null) {
        pages.remove()
        surfaceDriver.detach(page.session)
        continue
      }

      val current = page.target.configuration
      val configuration =
        current.copy(width = size.width.toDouble(), height = size.height.toDouble())
      if (configuration != current) {
        page.target =
          SurfaceTarget(
            page = page.target.page,
            key = SurfaceKey(nextSurfaceKey.addAndFetch(1)),
            configuration = configuration,
          )
        page.frame = null
        page.available = true
        page.failedRevision = null
        surfaceDriver.resize(page.session, configuration)
      }
      if (renderInvalidated || configuration != current) {
        page.requiredRevision = revision
      }
    }
  }

  private fun publicEvents(events: List<EditorEvent>): List<EditorEvent> = events.filterNot {
    it is EditorEvent.StateChanged || it is EditorEvent.RenderInvalidated
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
    val filtered = publicEvents(events)
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
          }
        }
      }
    }
  }

  internal suspend fun awaitPublished(revision: Long): EditorPublicationResult {
    val completion = CompletableDeferred<EditorPublicationResult>()
    val waiter = PublicationWaiter(revision, completion)
    val immediate =
      withContext(dispatcher) {
        mutex.withLock {
          ensureActive()
          val host = visualHost
          if (host == null) {
            NoHost
          } else {
            val current = published
            val pages = host.pages.values
            val targets = pages.map { it.target }
            if (
              current != null &&
                current.snapshot.version >= revision &&
                Publication.matchesTargets(current.frames, targets)
            ) {
              Published(current.snapshot.version)
            } else {
              pages
                .firstOrNull { page -> (page.failedRevision ?: -1L) >= revision }
                ?.let { page -> throw EditorSurfaceUnavailableException(page.target.page) }
              publicationWaiters.updatePersistent { it.adding(waiter) }
              null
            }
          }
        }
      }
    if (immediate != null) return immediate
    return try {
      completion.await()
    } finally {
      publicationWaiters.updatePersistent { it.removing(waiter) }
    }
  }

  internal fun activateVisualHost(token: Any) {
    runBlocking {
      mutex.withPriorityLock {
        ensureActive()
        val current = visualHost
        when {
          current == null -> visualHost = VisualHost(token)
          current.token === token -> return@withPriorityLock
          else -> error("Editor already has an active visual host")
        }
        publishIfReady()
      }
    }
  }

  internal fun deactivateVisualHost(token: Any) {
    scope.launch(dispatcher + NonCancellable, start = CoroutineStart.UNDISPATCHED) {
      // Queue teardown on the inner lock before returning. Otherwise a later synchronous
      // activation can pass the admission gate while the editor dispatcher is stalled.
      mutex.withPriorityLock(escalationMillis = 0) {
        if (visualHost?.token !== token) return@withPriorityLock
        visualHost = null
        refreshRetainedFrames()
        publicationWaiters.exchange(persistentListOf()).forEach { it.completion.complete(NoHost) }
      }
    }
  }

  internal fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
    wakeDelivery: (FrameKey) -> Unit,
  ): SurfaceSessionHandle = runBlocking {
    mutex.withPriorityLock {
      ensureActive()
      val host = visualHost ?: error("Cannot attach a surface without an active visual host")
      val configuration = SurfaceConfiguration(width, height, scaleFactor).withAppliedPageSize(page)
      val key = SurfaceKey(nextSurfaceKey.addAndFetch(1))
      val session = surfaceDriver.attach(this@Editor, page, handle, configuration)
      host.pages[page] =
        SurfacePage(
          target = SurfaceTarget(page, key, configuration),
          session = session,
          wakeDelivery = wakeDelivery,
          requiredRevision = appliedState.version,
        )
      startSurfaceRenders()
      publishIfReady()
      session
    }
  }

  internal suspend fun replaceSurface(
    session: SurfaceSessionHandle,
    configuration: SurfaceConfiguration,
  ) {
    if (session.isRetired) return
    withContext(dispatcher) {
      mutex.withLock {
        if (terminal) return@withLock
        if (session.isRetired) return@withLock
        val page = visualHost?.pages?.get(session.page) ?: return@withLock
        if (page.session !== session) {
          return@withLock
        }
        // The Compose effect is based on published geometry and may arrive after a
        // newer applied page size. Only its platform scale is current-independent.
        val currentConfiguration = configuration.withAppliedPageSize(session.page)
        if (page.target.configuration == currentConfiguration) return@withLock
        page.target =
          SurfaceTarget(
            page = session.page,
            key = SurfaceKey(nextSurfaceKey.addAndFetch(1)),
            configuration = currentConfiguration,
          )
        page.requiredRevision = appliedState.version
        page.frame = null
        page.available = true
        page.failedRevision = null
        surfaceDriver.resize(session, currentConfiguration)
        startSurfaceRenders()
        publishIfReady()
      }
    }
  }

  private fun SurfaceConfiguration.withAppliedPageSize(page: Int): SurfaceConfiguration {
    val size = appliedState.pageSizes.getOrNull(page) ?: return this
    return copy(width = size.width.toDouble(), height = size.height.toDouble())
  }

  internal fun detachSurface(session: SurfaceSessionHandle, onDetached: () -> Unit) {
    session.retire()
    scope.launch(dispatcher + NonCancellable, start = CoroutineStart.UNDISPATCHED) {
      try {
        mutex.withLock {
          if (terminal) return@withLock
          val host = visualHost
          if (host?.pages?.get(session.page)?.session === session) {
            host.pages.remove(session.page)
            publishIfReady()
          }
        }
      } finally {
        surfaceDriver.detach(session, onDetached)
      }
    }
  }

  internal fun deliverFrame(
    session: SurfaceSessionHandle,
    bitmap: ImageBitmap,
    pixelSize: IntSize,
    editorRevision: Long,
    frameKey: Long,
  ) {
    scope.launch(dispatcher) {
      mutex.withLock {
        if (terminal) return@withLock
        if (session.isRetired) return@withLock
        val page = visualHost?.pages?.get(session.page) ?: return@withLock
        if (page.session !== session) return@withLock
        val expectedRevision = page.inFlightRevision ?: return@withLock
        val deliveredFrameKey = FrameKey(frameKey)
        if (expectedRevision != editorRevision || page.preparedFrameKey != deliveredFrameKey) {
          page.inFlightRevision = null
          page.inFlightSurfaceKey = null
          page.preparedFrameKey = null
          startSurfaceRenders()
          return@withLock
        }

        page.inFlightRevision = null
        val surfaceKey = page.inFlightSurfaceKey
        page.inFlightSurfaceKey = null
        page.preparedFrameKey = null
        if (surfaceKey != page.target.key) {
          startSurfaceRenders()
          return@withLock
        }
        page.frame =
          PresentedFrame(
            bitmap = bitmap,
            pixelSize = pixelSize,
            proof =
              FrameProof(
                editorRevision = editorRevision,
                surfaceKey = page.target.key,
                frameKey = deliveredFrameKey,
              ),
          )
        publishIfReady()
        startSurfaceRenders()
      }
    }
  }

  internal fun retainedFrames(page: Int): List<ImageBitmap> =
    retainedFrameBitmaps.load()[page] ?: emptyList()

  internal fun surfaceDeliveryFailed(page: Int, session: SurfaceSessionHandle?, error: Throwable) {
    if (terminal) return
    scope.launch(dispatcher + NonCancellable, start = CoroutineStart.UNDISPATCHED) {
      val current = mutex.withPriorityLock {
        if (terminal) return@withPriorityLock false
        if (session?.isRetired == true) return@withPriorityLock false
        val host = visualHost ?: return@withPriorityLock false
        val currentSession = host.pages[page]?.session
        if (session == null) {
          currentSession == null && appliedState.pageSizes.getOrNull(page) != null
        } else {
          currentSession === session
        }
      }
      if (current) notifyFailure(error)
    }
  }

  internal fun surfaceUnavailable(session: SurfaceSessionHandle, frameKey: FrameKey) {
    scope.launch(dispatcher) {
      mutex.withLock {
        if (terminal) return@withLock
        if (session.isRetired) return@withLock
        val page = visualHost?.pages?.get(session.page) ?: return@withLock
        if (page.session !== session) return@withLock
        val failedRevision = page.inFlightRevision ?: return@withLock
        if (page.preparedFrameKey != frameKey) return@withLock

        page.inFlightRevision = null
        val surfaceKey = page.inFlightSurfaceKey
        page.inFlightSurfaceKey = null
        page.preparedFrameKey = null
        if (surfaceKey != page.target.key) {
          startSurfaceRenders()
          return@withLock
        }
        rejectFailedRevision(page, failedRevision)
        if (appliedState.version > failedRevision) {
          startSurfaceRenders()
        }
      }
    }
  }

  internal fun surfaceTargetUnavailable(
    session: SurfaceSessionHandle,
    frameKey: FrameKey,
    complete: ((Boolean) -> Unit)? = null,
  ) {
    scope.launch(dispatcher) {
      val accepted = mutex.withLock {
        if (terminal) return@withLock false
        if (session.isRetired) return@withLock false
        val page = visualHost?.pages?.get(session.page) ?: return@withLock false
        if (page.session !== session) return@withLock false
        if (page.inFlightRevision == null || page.preparedFrameKey != frameKey) {
          return@withLock false
        }

        page.inFlightRevision = null
        val surfaceKey = page.inFlightSurfaceKey
        page.inFlightSurfaceKey = null
        page.preparedFrameKey = null
        if (surfaceKey != page.target.key) {
          startSurfaceRenders()
          return@withLock false
        }

        page.target = page.target.copy(key = SurfaceKey(nextSurfaceKey.addAndFetch(1)))
        page.requiredRevision = appliedState.version
        page.frame = null
        page.available = false
        page.failedRevision = null
        refreshRetainedFrames()
        val error = EditorSurfaceUnavailableException(session.page)
        publicationWaiters.exchange(persistentListOf()).forEach {
          it.completion.completeExceptionally(error)
        }
        true
      }
      if (complete != null) {
        scope.launch(Dispatchers.Main) { complete(accepted) }
      }
    }
  }

  private fun startSurfaceRenders() {
    val host = visualHost ?: return
    for (page in host.pages.values) {
      val required = page.requiredRevision ?: continue
      if (!page.available || page.inFlightRevision != null) continue
      val proof = page.frame?.proof
      if (proof != null && Publication.accepts(proof, page.target, required, page.available)) {
        continue
      }

      val revision = Publication.workRevision(appliedState.version, required) ?: continue
      if ((page.failedRevision ?: -1L) >= revision) continue
      val target = page.target
      page.inFlightRevision = revision
      page.inFlightSurfaceKey = target.key
      page.preparedFrameKey = null
      surfaceDriver.render(page.session, revision) { frameKey ->
        onSurfacePrepared(page.session, target, revision, frameKey)
      }
    }
  }

  private fun onSurfacePrepared(
    session: SurfaceSessionHandle,
    target: SurfaceTarget,
    revision: Long,
    frameKey: FrameKey?,
  ) {
    scope.launch(dispatcher) {
      mutex.withLock {
        if (terminal) return@withLock
        if (session.isRetired) return@withLock
        val page = visualHost?.pages?.get(session.page) ?: return@withLock
        if (
          page.session !== session ||
            page.inFlightRevision != revision ||
            page.inFlightSurfaceKey != target.key ||
            page.preparedFrameKey != null
        ) {
          return@withLock
        }
        if (page.target != target) {
          page.inFlightRevision = null
          page.inFlightSurfaceKey = null
          startSurfaceRenders()
          return@withLock
        }
        if (frameKey == null) {
          page.inFlightRevision = null
          page.inFlightSurfaceKey = null
          if (appliedState.version > revision) {
            startSurfaceRenders()
          } else {
            rejectFailedRevision(page, revision)
          }
          return@withLock
        }
        val deliveredFrame = page.frame
        val deliveredProof = deliveredFrame?.proof
        if (
          deliveredProof != null &&
            deliveredProof.surfaceKey == target.key &&
            deliveredProof.frameKey == frameKey
        ) {
          page.inFlightRevision = null
          page.inFlightSurfaceKey = null
          page.frame = deliveredFrame.copy(proof = deliveredProof.copy(editorRevision = revision))
          publishIfReady()
          startSurfaceRenders()
          return@withLock
        }
        page.preparedFrameKey = frameKey
        page.wakeDelivery(frameKey)
      }
    }
  }

  private fun publishIfReady() {
    val host = visualHost
    if (host == null) {
      refreshRetainedFrames()
      return
    }
    val targets = host.pages.values.map { it.target }
    val targetsChanged = published?.let { !Publication.matchesTargets(it.frames, targets) } ?: false
    val pages =
      host.pages.values.map { page ->
        Publication.PageFacts(
          target = page.target,
          requiredRevision = page.requiredRevision,
          proof = page.frame?.proof,
          available = page.available,
        )
      }
    val publishedRevision = published?.snapshot?.version ?: 0L
    if (
      !Publication.canPublish(
        hasVisualHost = true,
        hasPublishedFrames = published?.frames?.isNotEmpty() == true,
        appliedRevision = appliedState.version,
        publishedRevision = publishedRevision,
        pages = pages,
        targetsChanged = targetsChanged,
      )
    ) {
      refreshRetainedFrames()
      return
    }

    val frames =
      host.pages.mapNotNull { (page, record) -> record.frame?.let { page to it } }.toMap()
    val bundle = PublishedBundle(snapshot = appliedState, frames = frames)
    published = bundle
    host.pages.values.forEach {
      it.requiredRevision = null
      it.failedRevision = null
    }
    refreshRetainedFrames()

    val completed = publicationWaiters.load().filter { it.revision <= bundle.snapshot.version }
    publicationWaiters.updatePersistent { it.removingAll(completed) }
    val result = Published(bundle.snapshot.version)
    completed.forEach { it.completion.complete(result) }
  }

  private fun refreshRetainedFrames() {
    val pages = mutableSetOf<Int>()
    published?.frames?.keys?.let(pages::addAll)
    visualHost?.pages?.keys?.let(pages::addAll)

    val frames = persistentMapOf<Int, PersistentList<ImageBitmap>>().builder()
    for (page in pages) {
      var bitmaps = persistentListOf<ImageBitmap>()
      published?.frames?.get(page)?.bitmap?.let { bitmaps = bitmaps.adding(it) }
      visualHost?.pages?.get(page)?.frame?.bitmap?.let { bitmap ->
        if (bitmaps.none { it === bitmap }) bitmaps = bitmaps.adding(bitmap)
      }
      if (bitmaps.isNotEmpty()) frames[page] = bitmaps
    }
    retainedFrameBitmaps.store(frames.build())
  }

  private fun rejectFailedRevision(page: SurfacePage, revision: Long) {
    page.failedRevision = maxOf(page.failedRevision ?: revision, revision)
    val failed = publicationWaiters.load().filter { it.revision <= revision }
    publicationWaiters.updatePersistent { it.removingAll(failed) }
    val error = EditorSurfaceUnavailableException(page.target.page)
    failed.forEach { it.completion.completeExceptionally(error) }
  }

  fun inspectState(options: InspectStateOptions? = null): String =
    withFailureNotification(defaultValue = { "" }) {
      ensureActive()
      inner.inspectState(options)
    }

  fun inspectStateAsMacro(): String =
    withFailureNotification(defaultValue = { "" }) {
      ensureActive()
      inner.inspectStateAsMacro()
    }

  fun inspectSelectionAsSliceMacro(): String? =
    withFailureNotification(defaultValue = { null }) {
      ensureActive()
      inner.inspectSelectionAsSliceMacro()
    }

  // Snapshot-based on purpose: these run synchronously inside touch dispatch on the
  // main thread, where a direct FFI call would contend on the Rust editor mutex and
  // stall input for as long as a background tick holds it (APP2-7P).
  fun selectionHitTest(page: Int, x: Float, y: Float): Boolean =
    appliedState.selectionHitRects.any { it.pageIdx == page && it.rect.containsPoint(x, y) }

  fun cursorHitTest(page: Int, x: Float, y: Float): Boolean =
    appliedState.cursorHitRects.any { it.pageIdx == page && it.rect.containsPoint(x, y) }

  fun interactiveHitTest(page: Int, x: Float, y: Float): InteractiveHit? {
    val region =
      appliedState.interactiveRegions.firstOrNull {
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
          if (terminal) {
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
          if (terminal) {
            null
          } else {
            inner.copySelection()
          }
        }
      }
    }

  internal suspend fun collectLocalChangesets(
    baseHeads: ByteArray?,
    block: EditorRequestScope.() -> Unit,
  ): EditorLocalChangesets {
    val resolvedBaseHeads = baseHeads ?: currentHeads()
    update(block = block)
    val changesets = localChangesetsSince(resolvedBaseHeads)
    val currentHeads = currentHeads()
    return EditorLocalChangesets(
      baseHeads = resolvedBaseHeads,
      currentHeads = currentHeads,
      changesets = changesets,
    )
  }

  internal suspend fun changesetIds(): List<String> = invokeCore { inner.changesetIds() }

  internal suspend fun missingChangesetsFor(remoteHeads: ByteArray): MissingBytes = invokeCore {
    val result = inner.missingChangesetsTolerant(remoteHeads)
    MissingBytes(bytes = result.bytes.toChangesetBytes(), withheld = result.withheld)
  }

  internal suspend fun splitChangesets(payload: ByteArray): List<SplitChangeset> = invokeCore {
    inner.splitChangesets(payload).map {
      SplitChangeset(id = it.id, bytes = it.bytes.toChangesetBytes())
    }
  }

  internal suspend fun partitionRemoteChangesets(payload: ByteArray): PartitionedBytes =
    invokeCore {
      val partitioned = inner.partitionRemoteChangesets(payload)
      PartitionedBytes(
        ready = partitioned.ready.toChangesetBytes(),
        blocked = partitioned.blocked.toChangesetBytes(),
      )
    }

  internal suspend fun receiveRemoteChangeset(payload: ByteArray) {
    withSuspendFailureNotification {
      val events =
        withContext(dispatcher) {
          mutex.withLock {
            ensureActive()
            inner.receiveRemoteChangeset(payload)
            val tick = inner.tick() ?: return@withLock emptyList()
            install(tick)
            publicEvents(tick.events)
          }
        }
      emit(events)
    }
  }

  suspend fun insertTemplateFragment(changesets: ByteArray): Boolean =
    try {
      localEdits.run {
        val events =
          withContext(dispatcher) {
            mutex.withLock {
              ensureActive()
              inner.insertTemplateFragment(changesets)
              val tick = inner.tick() ?: return@withLock emptyList()
              install(tick)
              publicEvents(tick.events)
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

  internal suspend fun currentHeads(): ByteArray = invokeCore { inner.currentHeads() }

  private suspend fun localChangesetsSince(remoteHeads: ByteArray): ByteArray = invokeCore {
    inner.localChangesetsSince(remoteHeads)
  }

  fun dispose() {
    if (!disposed.compareAndSet(expectedValue = false, newValue = true)) return
    EditorRegistry.unregisterAsync(this)
    val error = CancellationException("Editor disposed")
    listeners.store(persistentMapOf())
    surfaceDriver.dispose()
    scope.launch(dispatcher + NonCancellable, start = CoroutineStart.UNDISPATCHED) {
      var receipts: Collection<CompletableDeferred<EditorUpdate>> = emptyList()
      var waiters: List<PublicationWaiter> = emptyList()
      var edits: List<LocalEdit> = emptyList()
      mutex.withPriorityLock {
        receipts = requestReceipts.exchange(persistentMapOf()).values
        waiters = publicationWaiters.exchange(persistentListOf())
        edits = queuedLocalEdits.exchange(persistentListOf())
        visualHost = null
        retainedFrameBitmaps.store(persistentMapOf())
      }
      receipts.forEach { it.completeExceptionally(error) }
      waiters.forEach { it.completion.completeExceptionally(error) }
      edits.forEach(LocalEdit::complete)
    }
  }

  private fun readSnapshot(version: Long, events: List<EditorEvent>): EditorState {
    val previous = appliedState
    val initial = previous.version == 0L
    val changedFields =
      events.filterIsInstance<EditorEvent.StateChanged>().flatMapTo(mutableSetOf()) { it.fields }
    fun changed(field: StateField): Boolean = initial || field in changedFields

    val documentChanged = StateField.Doc in changedFields
    val pageSizesChanged = changed(StateField.PageSizes)
    val selectionFieldChanged = changed(StateField.Selection)
    val selection =
      if (selectionFieldChanged || documentChanged) inner.selection() else previous.selection
    val selectionChanged = previous.selection != selection
    val renderInvalidated = events.any { it is EditorEvent.RenderInvalidated }
    val hitGeometryStale =
      selectionChanged || documentChanged || pageSizesChanged || renderInvalidated || initial
    val selectionEndpoints =
      when {
        selection == null || selection.anchor == selection.head -> null
        hitGeometryStale || changed(StateField.Cursor) -> inner.selectionEndpoints()
        else -> previous.selectionEndpoints
      }
    val selectionHitRects =
      when {
        selection == null || selection.anchor == selection.head -> emptyList()
        hitGeometryStale -> inner.selectionHitRects()
        else -> previous.selectionHitRects
      }
    val cursorHitRects =
      when {
        selection == null || selection.anchor != selection.head -> emptyList()
        hitGeometryStale -> inner.cursorHitRects()
        else -> previous.cursorHitRects
      }
    val interactiveRegions =
      if (documentChanged || pageSizesChanged || renderInvalidated || initial) {
        inner.interactiveRegions()
      } else {
        previous.interactiveRegions
      }
    val ime =
      when {
        !imeSessionActive.load() -> null
        changed(StateField.Ime) || previous.ime == null ->
          inner.ime(IME_SNAPSHOT_WINDOW, IME_SNAPSHOT_WINDOW)
        else -> previous.ime
      }
    val placeholder =
      if (changed(StateField.Placeholder)) {
        inner.placeholder()
      } else {
        previous.placeholder
      }
    val trackedRanges =
      if (changed(StateField.TrackedRanges)) {
        inner.trackedRanges(null)
      } else {
        previous.trackedRanges
      }
    val tableOverlays =
      if (changed(StateField.TableOverlays)) {
        inner.tableOverlays()
      } else {
        previous.tableOverlays
      }
    val snapshot =
      EditorState(
        version = version,
        documentRevision = previous.documentRevision + if (documentChanged) 1L else 0L,
        cursor =
          if (changed(StateField.Cursor) || documentChanged || renderInvalidated) {
            inner.cursor()
          } else {
            previous.cursor
          },
        placeholder = placeholder,
        selection = selection,
        selectionEndpoints = selectionEndpoints,
        pageSizes = if (pageSizesChanged) inner.pageSizes() else previous.pageSizes,
        externalElements =
          if (changed(StateField.ExternalElements)) {
            inner.externalElements()
          } else {
            previous.externalElements
          },
        tableOverlays = tableOverlays,
        rootAttrs = if (changed(StateField.RootAttrs)) inner.rootAttrs() else previous.rootAttrs,
        rootModifiers =
          if (
            changed(StateField.RootAttrs) ||
              changed(StateField.Modifiers) ||
              changed(StateField.Block)
          ) {
            inner.rootModifiers()
          } else {
            previous.rootModifiers
          },
        modifierState =
          if (changed(StateField.Modifiers)) inner.modifierState() else previous.modifierState,
        blockState = if (changed(StateField.Block)) inner.blockState() else previous.blockState,
        ime = ime,
        lastHistoryTag =
          if (changed(StateField.LastHistoryTag)) {
            inner.lastHistoryTag()
          } else {
            previous.lastHistoryTag
          },
        trackedRanges = trackedRanges,
        trackedRangesContainingSelectionHead =
          if (selection != null && selection.anchor == selection.head) {
            if (
              selectionChanged ||
                changed(StateField.TrackedRanges) ||
                documentChanged ||
                renderInvalidated
            ) {
              inner.trackedRangesContainingPosition(selection.head, null)
            } else {
              previous.trackedRangesContainingSelectionHead
            }
          } else {
            emptyList()
          },
        selectionHitRects = selectionHitRects,
        cursorHitRects = cursorHitRects,
        interactiveRegions = interactiveRegions,
      )
    appliedState = snapshot
    return snapshot
  }

  private fun ensureActive() {
    if (disposed.load()) {
      throw CancellationException("Editor disposed")
    }
    check(!failed.load()) { "Editor failed" }
  }

  internal fun fail(error: Throwable) {
    if (error is CancellationException) {
      throw error
    }
    if (disposed.load()) return
    if (!failed.compareAndSet(expectedValue = false, newValue = true)) return

    EditorRegistry.unregisterAsync(this)
    scope.launch(dispatcher + NonCancellable, start = CoroutineStart.UNDISPATCHED) {
      var receipts: Collection<CompletableDeferred<EditorUpdate>> = emptyList()
      var waiters: List<PublicationWaiter> = emptyList()
      var edits: List<LocalEdit> = emptyList()
      mutex.withPriorityLock {
        receipts = requestReceipts.exchange(persistentMapOf()).values
        waiters = publicationWaiters.exchange(persistentListOf())
        edits = queuedLocalEdits.exchange(persistentListOf())
      }
      receipts.forEach { it.completeExceptionally(error) }
      waiters.forEach { it.completion.completeExceptionally(error) }
      edits.forEach { it.fail(error) }
      onError(this@Editor, error)
    }
  }

  private fun notifyFailure(error: Throwable) = fail(error)

  private fun <T> withFailureNotification(defaultValue: () -> T, block: () -> T): T =
    try {
      block()
    } catch (e: CancellationException) {
      throw e
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
          ensureActive()
          block(inner)
        }
      }
    }

  private suspend fun <T> invokeCore(block: () -> T): T =
    try {
      withContext(dispatcher) {
        mutex.withLock {
          ensureActive()
          block()
        }
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      fail(e)
      throw e
    }

  private suspend fun withSuspendFailureNotification(block: suspend () -> Unit) {
    try {
      block()
    } catch (e: CancellationException) {
      throw e
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
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      notifyFailure(e)
      defaultValue()
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
            val initialized = AtomicBoolean(false)
            val editor =
              Editor(createInner(), scope, dispatcher) { failedEditor, error ->
                if (initialized.load()) onError(failedEditor, error)
              }
            createdEditor = editor

            editor.on<EditorEvent.FontDataMissing>(FontLoader.fontDataMissingHandler)
            EditorRegistry.register(editor)
            try {
              EditorRegistry.commitResourceUpdate {
                PlatformModule.editorHost.setThemeVariant(themeVariant)
              }
              checkNotNull(editor.update { enqueue(Message.System(SystemEvent.Initialize)) })
              initialized.store(true)
            } finally {
              if (!initialized.load()) {
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
