package co.typie.editor

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import co.touchlab.kermit.Logger
import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.ClipboardPayload
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainDoc
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.ThemeVariant
import co.typie.editor.ffi.Viewport
import co.typie.platform.PlatformModule
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
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
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

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

  val cursor: CursorMetrics? by derivedStateOf { state.cursor }
  val selection: Selection? by derivedStateOf { state.selection }
  val pageSizes: List<Size> by derivedStateOf { state.pageSizes }
  val externalElements: List<ExternalElement> by derivedStateOf { state.externalElements }
  val rootAttrs: PlainRootNode? by derivedStateOf { state.rootAttrs }
  val rootModifiers: List<EditorModifier>? by derivedStateOf { state.rootModifiers }
  val modifierState: ModifierState? by derivedStateOf { state.modifierState }
  val blockState: BlockState? by derivedStateOf { state.blockState }
  val ime: Ime? by derivedStateOf { state.ime }

  private val mutex: Mutex = Mutex()
  private val versionCounter: AtomicLong = AtomicLong(0L)
  private val disposed: AtomicBoolean = AtomicBoolean(false)
  private val syncInProgress: AtomicBoolean = AtomicBoolean(false)
  private val attachedPages: AtomicReference<PersistentSet<Int>> =
    AtomicReference(persistentSetOf())
  private val pendingSettles: AtomicReference<PersistentList<PendingSettle>> =
    AtomicReference(persistentListOf())
  private val queued: AtomicBoolean = AtomicBoolean(false)

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

  suspend fun await(beforeCommit: ((EditorState) -> Unit)? = null, block: EditorScope.() -> Unit) {
    withSuspendFailureNotification { awaitOrThrow(beforeCommit = beforeCommit, block = block) }
  }

  private suspend fun awaitOrThrow(
    beforeCommit: ((EditorState) -> Unit)? = null,
    block: EditorScope.() -> Unit,
  ) {
    val messages = mutableListOf<Message>()
    val collector =
      object : EditorScope {
        override fun enqueue(message: Message) {
          messages += message
        }
      }
    block(collector)
    if (messages.isEmpty()) return

    val (events, snapshot) =
      withContext(NonCancellable + dispatcher) {
        mutex.withLock {
          if (disposed.load()) throw CancellationException("Editor disposed")
          for (m in messages) inner.enqueue(m)
          val e = inner.tick()
          val version = versionCounter.addAndFetch(1L)
          val s = readSnapshot(version)
          e to s
        }
      }

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
  }

  fun sync(beforeCommit: ((EditorState) -> Unit)? = null, block: EditorScope.() -> Unit) {
    if (!syncInProgress.compareAndSet(expectedValue = false, newValue = true)) {
      notifyFailure(IllegalStateException("nested sync is not supported"))
      return
    }
    try {
      withFailureNotification {
        runBlocking {
          val events: List<EditorEvent>
          mutex.withLock {
            if (disposed.load()) error("Editor disposed")
            val collector =
              object : EditorScope {
                override fun enqueue(message: Message) {
                  inner.enqueue(message)
                }
              }
            block(collector)
            events = inner.tick()
            val version = versionCounter.addAndFetch(1L)
            val snapshot = readSnapshot(version)
            beforeCommit?.invoke(snapshot)
            commit(snapshot)
          }
          emit(events)
        }
      }
    } finally {
      syncInProgress.store(false)
    }
  }

  fun enqueue(message: Message) {
    if (disposed.load()) return
    inner.enqueue(message)
    scheduleTick()
  }

  suspend fun can(message: Message): Boolean =
    withSuspendFailureNotification(defaultValue = { false }) {
      withContext(dispatcher) {
        mutex.withLock {
          if (disposed.load()) {
            false
          } else {
            inner.can(message)
          }
        }
      }
    }

  private fun scheduleTick() {
    if (!queued.compareAndSet(expectedValue = false, newValue = true)) return
    scope.launch(dispatcher) {
      withSuspendFailureNotification {
        val events =
          withContext(dispatcher) {
            mutex.withLock {
              queued.store(false)
              if (disposed.load()) return@withLock emptyList()
              val e = inner.tick()
              val version = versionCounter.addAndFetch(1L)
              commit(readSnapshot(version))
              e
            }
          }
        emit(events)
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

  fun attachSurface(page: Int, handle: Long, width: Double, height: Double, scaleFactor: Double) {
    attachedPages.updatePersistent { it.add(page) }
    try {
      inner.attachSurface(page, handle, width, height, scaleFactor)
    } catch (e: Throwable) {
      attachedPages.updatePersistent { it.remove(page) }
      notifyFailure(e)
    }
  }

  fun detachSurface(page: Int) {
    attachedPages.updatePersistent { it.remove(page) }
    withFailureNotification {
      inner.detachSurface(page)
      pendingSettles.load().forEach { it.markDetached(page) }
    }
  }

  fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    withFailureNotification {
      inner.resizeSurface(page, width, height, scaleFactor)
    }

  fun renderSurface(page: Int): Long = runBlocking {
    withSuspendFailureNotification(defaultValue = { versionCounter.load() }) {
      mutex.withLock {
        inner.renderSurface(page)
        versionCounter.load()
      }
    }
  }

  fun onPageSettled(page: Int, version: Long) {
    pendingSettles.load().forEach { it.markCommitted(page, version) }
  }

  fun inspectState(options: InspectStateOptions? = null): String = inner.inspectState(options)

  fun inspectStateAsMacro(): String = inner.inspectStateAsMacro()

  fun ime(beforeLimit: Int, afterLimit: Int): Ime = inner.ime(beforeLimit, afterLimit)

  fun selectionHitTest(page: Int, x: Float, y: Float): Boolean = inner.selectionHitTest(page, x, y)

  fun cursorHitTest(page: Int, x: Float, y: Float): Boolean = inner.cursorHitTest(page, x, y)

  fun selectionEndpoints(): SelectionEndpoints? = inner.selectionEndpoints()

  fun copySelection(): ClipboardPayload? = inner.copySelection()

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

  internal suspend fun receiveRemoteChangeset(payload: ByteArray) {
    withSuspendFailureNotification {
      val events =
        withContext(NonCancellable + dispatcher) {
          mutex.withLock {
            if (disposed.load()) throw CancellationException("Editor disposed")
            inner.receiveRemoteChangeset(payload)
            val e = inner.tick()
            val version = versionCounter.addAndFetch(1L)
            commit(readSnapshot(version))
            e
          }
        }
      emit(events)
    }
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
    listeners.store(persistentMapOf())
    attachedPages.store(persistentSetOf())
  }

  private fun readSnapshot(version: Long): EditorState =
    EditorState(
      version = version,
      cursor = inner.cursor(),
      selection = inner.selection(),
      pageSizes = inner.pageSizes(),
      externalElements = inner.externalElements(),
      rootAttrs = inner.rootAttrs(),
      rootModifiers = inner.rootModifiers(),
      modifierState = inner.modifierState(),
      blockState = inner.blockState(),
      ime = inner.ime(Int.MAX_VALUE, Int.MAX_VALUE),
    )

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

    private suspend fun createInitialized(
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
            PlatformModule.editorHost.setThemeVariant(themeVariant)
            editor.awaitOrThrow {
              enqueue(Message.System(SystemEvent.ThemeVariantChanged))
              enqueue(Message.System(SystemEvent.Initialize))
            }
            editor
          }

        EditorRegistry.register(editor)
        createdEditor = null
        editor
      } finally {
        createdEditor?.dispose()
      }
    }
  }
}
