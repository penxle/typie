package co.typie.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.CursorRect
import co.typie.editor.ffi.Doc
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
import kotlin.reflect.KClass
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class Editor
private constructor(private val inner: co.typie.editor.ffi.Editor, val scope: CoroutineScope) {
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

  internal val pageOffsets = mutableStateMapOf<Int, Offset>()

  private var queued = false
  private var batching = false

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

  internal inline fun batch(block: () -> Unit) {
    batching = true
    block()
    batching = false
    tick()
  }

  private fun tick() {
    queued = false
    val events = inner.tick()
    for (event in events) {
      emit(event)
    }
  }

  private val stateChangedHandler: EditorEventListener<EditorEvent.StateChanged> = { _, event ->
    for (field in event.fields) {
      when (field) {
        StateField.Doc -> {}
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

  fun localToGlobal(page: Int, x: Float, y: Float): Offset? = localToGlobal(page, x, y, pageOffsets)

  fun globalToLocal(x: Float, y: Float): PagePoint? = globalToLocal(x, y, pageOffsets, pageSizes)

  fun attachSurface(page: Int, handle: Long, width: Double, height: Double, scaleFactor: Double) =
    inner.attachSurface(page, handle, width, height, scaleFactor)

  fun detachSurface(page: Int) = inner.detachSurface(page)

  fun resizeSurface(page: Int, width: Double, height: Double, scaleFactor: Double) =
    inner.resizeSurface(page, width, height, scaleFactor)

  fun renderSurface(page: Int) = inner.renderSurface(page)

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
        FontLoader.initFonts()

        val inner = PlatformModule.editorHost.createEditor(doc, selection, viewport)
        val editor = Editor(inner, scope)

        editor.on<EditorEvent.StateChanged>(editor.stateChangedHandler)
        editor.on<EditorEvent.FontManifestMissing>(FontLoader.fontManifestMissingHandler)
        editor.on<EditorEvent.FontDataMissing>(FontLoader.fontDataMissingHandler)

        editor.enqueue(Message.System(SystemEvent.Initialize))

        editor
      }
  }
}
