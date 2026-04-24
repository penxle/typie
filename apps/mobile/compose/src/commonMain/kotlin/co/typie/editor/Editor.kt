package co.typie.editor

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import co.touchlab.kermit.Logger
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import co.typie.platform.PlatformModule
import io.sentry.kotlin.multiplatform.Sentry
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
) {
  var state: EditorState by mutableStateOf(EditorState.Initial)
    private set

  val cursor: CursorMetrics? by derivedStateOf { state.cursor }
  val selection: Selection? by derivedStateOf { state.selection }
  val pageSizes: List<Size> by derivedStateOf { state.pageSizes }
  val documentAttrs: DocumentAttrs? by derivedStateOf { state.documentAttrs }
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

  fun focus() = focusRequester.requestFocus()

  fun blur() {
    focusManager?.clearFocus()
  }

  fun deactivateScene() {
    focusManager?.clearFocus()
  }

  suspend fun await(block: EditorScope.() -> Unit) {
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
      withContext(NonCancellable) { mutex.withLock { if (!disposed.load()) commit(snapshot) } }
    }
  }

  fun sync(block: EditorScope.() -> Unit) {
    if (!syncInProgress.compareAndSet(expectedValue = false, newValue = true)) {
      error("nested sync is not supported")
    }
    try {
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
          commit(readSnapshot(version))
        }
        emit(events)
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

  private fun scheduleTick() {
    if (!queued.compareAndSet(expectedValue = false, newValue = true)) return
    scope.launch(dispatcher) {
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
            Sentry.captureException(e)
          }
        }
      }
    }
  }

  fun attachSurface(page: Int, handle: Long, width: Double, height: Double, scaleFactor: Double) {
    attachedPages.updatePersistent { it.add(page) }
    inner.attachSurface(page, handle, width, height, scaleFactor)
  }

  fun detachSurface(page: Int) {
    attachedPages.updatePersistent { it.remove(page) }
    inner.detachSurface(page)
    pendingSettles.load().forEach { it.markDetached(page) }
  }

  fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    inner.resizeSurface(page, width, height, scaleFactor)

  fun renderSurface(page: Int): Long = runBlocking {
    mutex.withLock {
      inner.renderSurface(page)
      versionCounter.load()
    }
  }

  fun onPageSettled(page: Int, version: Long) {
    pendingSettles.load().forEach { it.markCommitted(page, version) }
  }

  fun inspectState(options: InspectStateOptions? = null): String = inner.inspectState(options)

  fun inspectStateAsMacro(): String = inner.inspectStateAsMacro()

  fun ime(beforeLimit: Int, afterLimit: Int): Ime = inner.ime(beforeLimit, afterLimit)

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
      selection = runCatching { inner.selection() }.getOrNull(),
      pageSizes = inner.pageSizes(),
      documentAttrs = runCatching { inner.documentAttrs() }.getOrNull(),
      ime = runCatching { inner.ime(Int.MAX_VALUE, Int.MAX_VALUE) }.getOrNull(),
    )

  private fun commit(snapshot: EditorState) {
    if (snapshot.version <= state.version) return
    state = snapshot
  }

  companion object {
    suspend fun create(
      doc: Doc,
      selection: Selection,
      viewport: Viewport,
      scope: CoroutineScope,
      dispatcher: CoroutineDispatcher = Dispatchers.Default.limitedParallelism(1),
    ): Editor =
      withContext(Dispatchers.Default) {
        val inner = PlatformModule.editorHost.createEditor(doc, selection, viewport)
        val editor = Editor(inner, scope, dispatcher)

        editor.on<EditorEvent.FontDataMissing>(FontLoader.fontDataMissingHandler)

        try {
          editor.await { enqueue(Message.System(SystemEvent.Initialize)) }
        } catch (e: Throwable) {
          editor.dispose()
          throw e
        }

        EditorRegistry.register(editor)
        editor
      }
  }
}
