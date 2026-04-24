package co.typie.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import co.typie.platform.PlatformModule
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.reflect.KClass
import kotlinx.collections.immutable.PersistentList
import kotlinx.collections.immutable.PersistentMap
import kotlinx.collections.immutable.PersistentSet
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.persistentSetOf
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.suspendCancellableCoroutine
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
  var documentAttrs by mutableStateOf<DocumentAttrs?>(null)
    private set

  var cursor by mutableStateOf<CursorMetrics?>(null)
    private set

  var selection by mutableStateOf<Selection?>(null)
    private set

  var pageSizes by mutableStateOf<List<Size>>(emptyList())
    private set

  var ime by mutableStateOf<Ime?>(null)
    private set

  internal val focusRequester = FocusRequester()
  internal var focusManager: FocusManager? = null

  private val queued = AtomicBoolean(false)
  private val syncing = AtomicBoolean(false)
  private val disposed = AtomicBoolean(false)
  private val dispatches =
    AtomicReference<PersistentList<CancellableContinuation<Unit>>>(persistentListOf())
  private val mutex = Mutex()

  @PublishedApi
  internal val listeners =
    AtomicReference<
      PersistentMap<KClass<out EditorEvent>, PersistentSet<(Editor, EditorEvent) -> Unit>>
    >(
      persistentMapOf()
    )

  inline fun <reified T : EditorEvent> on(noinline listener: EditorEventListener<T>): () -> Unit {
    @Suppress("UNCHECKED_CAST") val wrapped = listener as (Editor, EditorEvent) -> Unit
    val key = T::class
    listeners.update { map ->
      val set = map[key] ?: persistentSetOf()
      map.put(key, set.add(wrapped))
    }
    return {
      listeners.update { map ->
        val set = map[key] ?: return@update map
        map.put(key, set.remove(wrapped))
      }
    }
  }

  private fun emit(event: EditorEvent) {
    val snapshot = listeners.load()[event::class] ?: return
    snapshot.forEach { it(this, event) }
  }

  fun enqueue(message: Message) {
    if (disposed.load() || !scope.isActive) return
    inner.enqueue(message)
    if (!syncing.load() && queued.compareAndSet(false, true)) {
      try {
        scope.launch(dispatcher) {
          val events = mutex.withLock { runLockedTick() }
          for (event in events) emit(event)
        }
      } catch (_: CancellationException) {
        // scope was cancelled between isActive check and launch — race, just ignore
      }
    }
  }

  fun resizeViewport(width: Float, height: Float, scaleFactor: Double) {
    enqueue(
      Message.System(SystemEvent.Resize(width = width, height = height, scaleFactor = scaleFactor))
    )
  }

  suspend fun dispatch(vararg messages: Message) {
    if (messages.isEmpty()) {
      return
    }

    suspendCancellableCoroutine { cont ->
      dispatches.update { it.add(cont) }
      cont.invokeOnCancellation { dispatches.update { it.remove(cont) } }
      if (disposed.load()) {
        cont.cancel()
        return@suspendCancellableCoroutine
      }
      messages.forEach(::enqueue)
    }
  }

  fun sync(block: Editor.() -> Unit) {
    syncing.store(true)
    try {
      block(this)
    } finally {
      syncing.store(false)
    }
    val events = runBlocking { mutex.withLock { runLockedTick() } }
    for (event in events) emit(event)
  }

  private fun runLockedTick(): List<EditorEvent> {
    queued.store(false)
    val waiters = drainDispatches()

    val events =
      try {
        inner.tick()
      } catch (e: Throwable) {
        waiters.forEach { it.resumeWithException(e) }
        return emptyList()
      }

    waiters.forEach { it.resume(Unit) }
    return events
  }

  private val stateChangedHandler: EditorEventListener<EditorEvent.StateChanged> = { _, event ->
    for (field in event.fields) {
      when (field) {
        StateField.Doc -> {}
        StateField.DocAttrs -> documentAttrs = inner.documentAttrs()
        StateField.Cursor -> cursor = inner.cursor()
        StateField.Selection -> selection = inner.selection()
        StateField.PageSizes -> pageSizes = inner.pageSizes()
        StateField.Ime -> ime = inner.ime(Int.MAX_VALUE, Int.MAX_VALUE)
        StateField.Modifiers -> {}
      }
    }
  }

  fun focus() = focusRequester.requestFocus()

  fun blur() {
    focusManager?.clearFocus()
  }

  fun deactivateScene() {
    focusManager?.clearFocus()
  }

  fun dispose() {
    if (!disposed.compareAndSet(false, true)) return
    EditorRegistry.unregisterAsync(this)
    val waiters = drainDispatches()
    waiters.forEach { it.cancel() }
  }

  fun attachSurface(page: Int, handle: Long, width: Double, height: Double, scaleFactor: Double) =
    inner.attachSurface(page, handle, width, height, scaleFactor)

  fun detachSurface(page: Int) = inner.detachSurface(page)

  fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    inner.resizeSurface(page, width, height, scaleFactor)

  fun renderSurface(page: Int) = inner.renderSurface(page)

  fun inspectState(options: InspectStateOptions? = null): String = inner.inspectState(options)

  fun inspectStateAsMacro(): String = inner.inspectStateAsMacro()

  fun ime(beforeLimit: Int, afterLimit: Int): Ime = inner.ime(beforeLimit, afterLimit)

  @PublishedApi
  internal inline fun <T> AtomicReference<T>.update(transform: (T) -> T): T {
    while (true) {
      val current = load()
      val next = transform(current)
      if (compareAndSet(current, next)) return next
    }
  }

  private fun drainDispatches(): List<CancellableContinuation<Unit>> {
    while (true) {
      val current = dispatches.load()
      if (current.isEmpty()) return emptyList()
      if (dispatches.compareAndSet(current, persistentListOf())) return current
    }
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

        editor.on<EditorEvent.StateChanged>(editor.stateChangedHandler)
        editor.on<EditorEvent.FontDataMissing>(FontLoader.fontDataMissingHandler)

        EditorRegistry.register(editor)

        editor.enqueue(Message.System(SystemEvent.Initialize))

        editor
      }
  }
}
