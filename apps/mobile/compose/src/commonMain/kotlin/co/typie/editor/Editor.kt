package co.typie.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import co.typie.editor.ffi.CursorRect
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
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.reflect.KClass
import kotlinx.coroutines.CancellableContinuation
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext

class Editor
internal constructor(private val inner: co.typie.editor.ffi.Editor, val scope: CoroutineScope) {
  var documentAttrs by mutableStateOf<DocumentAttrs?>(null)
    private set

  var cursor by mutableStateOf<CursorRect?>(null)
    private set

  var selection by mutableStateOf<Selection?>(null)
    private set

  var pageSizes by mutableStateOf<List<Size>>(emptyList())
    private set

  var ime by mutableStateOf<Ime?>(null)
    private set

  internal val focusRequester = FocusRequester()
  internal var focusManager: FocusManager? = null

  private var queued = false
  private var batching = false
  private val dispatches = mutableListOf<CancellableContinuation<Unit>>()

  @PublishedApi
  internal val listeners =
    mutableMapOf<KClass<out EditorEvent>, MutableSet<(Editor, EditorEvent) -> Unit>>()

  inline fun <reified T : EditorEvent> on(noinline listener: EditorEventListener<T>): () -> Unit {
    @Suppress("UNCHECKED_CAST") val wrapped = listener as (Editor, EditorEvent) -> Unit
    val set = listeners.getOrPut(T::class) { mutableSetOf() }
    set.add(wrapped)
    return { set.remove(wrapped) }
  }

  private fun emit(event: EditorEvent) {
    listeners[event::class]?.forEach { it(this, event) }
  }

  fun enqueue(message: Message) {
    inner.enqueue(message)
    if (!batching && !queued) {
      queued = true
      scope.launch(Dispatchers.Main) { tick() }
    }
  }

  suspend fun dispatch(vararg messages: Message) {
    if (messages.isEmpty()) {
      return
    }

    withContext(Dispatchers.Main.immediate) {
      suspendCancellableCoroutine { cont ->
        dispatches.add(cont)
        cont.invokeOnCancellation { scope.launch(Dispatchers.Main) { dispatches.remove(cont) } }
        messages.forEach(::enqueue)
      }
    }
  }

  internal inline fun batch(block: () -> Unit) {
    batching = true
    block()
    batching = false
    tick()
  }

  private fun tick() {
    queued = false
    val waiters = dispatches.toList()
    dispatches.clear()

    val events =
      try {
        inner.tick()
      } catch (e: Throwable) {
        waiters.forEach { it.resumeWithException(e) }
        return
      }

    for (event in events) {
      emit(event)
    }

    waiters.forEach { it.resume(Unit) }
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
    EditorRegistry.unregisterAsync(this)
    val waiters = dispatches.toList()
    dispatches.clear()
    waiters.forEach { it.cancel() }
  }

  fun attachSurface(page: Int, handle: Long, width: Double, height: Double, scaleFactor: Double) =
    inner.attachSurface(page, handle, width, height, scaleFactor)

  fun detachSurface(page: Int) = inner.detachSurface(page)

  fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    inner.resizeSurface(page, width, height, scaleFactor)

  fun renderSurface(page: Int) = inner.renderSurface(page)

  fun resizeViewport(width: Float, height: Float, scaleFactor: Double) {
    enqueue(
      Message.System(SystemEvent.Resize(width = width, height = height, scaleFactor = scaleFactor))
    )
  }

  fun inspectState(options: InspectStateOptions? = null): String = inner.inspectState(options)

  fun inspectStateAsMacro(): String = inner.inspectStateAsMacro()

  fun ime(beforeLimit: Int, afterLimit: Int): Ime = inner.ime(beforeLimit, afterLimit)

  companion object {
    suspend fun create(
      doc: Doc,
      selection: Selection,
      viewport: Viewport,
      scope: CoroutineScope,
    ): Editor =
      withContext(Dispatchers.Default) {
        val inner = PlatformModule.editorHost.createEditor(doc, selection, viewport)
        val editor = Editor(inner, scope)

        editor.on<EditorEvent.StateChanged>(editor.stateChangedHandler)
        editor.on<EditorEvent.FontDataMissing>(FontLoader.fontDataMissingHandler)

        EditorRegistry.register(editor)

        editor.enqueue(Message.System(SystemEvent.Initialize))

        editor
      }
  }
}
