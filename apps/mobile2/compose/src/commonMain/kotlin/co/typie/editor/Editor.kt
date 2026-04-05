package co.typie.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.InputContext
import co.typie.editor.ffi.InspectStateOptions
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import org.koin.core.component.KoinComponent
import org.koin.core.component.get
import kotlin.reflect.KClass

class Editor private constructor(
  private val inner: co.typie.editor.ffi.Editor,
  val scope: CoroutineScope,
) {
  var cursor by mutableStateOf<PageRect?>(null)
    private set

  var selection by mutableStateOf<Selection?>(null)
    private set

  var pageSizes by mutableStateOf<List<Size>>(emptyList())
    private set

  private var queued = false

  @PublishedApi
  internal val listeners =
    mutableMapOf<KClass<out EditorEvent>, MutableSet<(Editor, EditorEvent) -> Unit>>()

  inline fun <reified T : EditorEvent> on(noinline listener: EditorEventListener<T>): () -> Unit {
    @Suppress("UNCHECKED_CAST")
    val wrapped = listener as (Editor, EditorEvent) -> Unit
    val set = listeners.getOrPut(T::class) { mutableSetOf() }
    set.add(wrapped)
    return { set.remove(wrapped) }
  }

  private fun emit(event: EditorEvent) {
    listeners[event::class]?.forEach { it(this, event) }
  }


  fun enqueue(message: Message) {
    inner.enqueue(message)
    if (!queued) {
      queued = true
      scope.launch { tick() }
    }
  }

  private fun tick() {
    queued = false

    val events = inner.tick()
    for (event in events) {
      emit(event)
    }
  }

  private val stateChangedHandler: EditorEventListener<EditorEvent.StateChanged> =
    { _, event ->
      for (field in event.fields) {
        when (field) {
          StateField.Cursor -> cursor = inner.cursor()
          StateField.Selection -> selection = inner.selection()
          StateField.PageSizes -> pageSizes = inner.pageSizes()
          else -> {}
        }
      }
    }

  fun attachSurface(page: Int, handle: Long, width: Int, height: Int, scaleFactor: Double) =
    inner.attachSurface(page, handle, width, height, scaleFactor)

  fun detachSurface(page: Int) = inner.detachSurface(page)

  fun resizeSurface(page: Int, width: Int, height: Int, scaleFactor: Double) =
    inner.resizeSurface(page, width, height, scaleFactor)

  fun renderSurface(page: Int) = inner.renderSurface(page)

  fun inspectState(options: InspectStateOptions? = null): String = inner.inspectState(options)

  fun inspectStateAsMacro(): String = inner.inspectStateAsMacro()

  fun inputContext(beforeLimit: Int, afterLimit: Int): InputContext =
    inner.inputContext(beforeLimit, afterLimit)

  companion object : KoinComponent {
    suspend fun create(
      doc: Doc,
      selection: Selection,
      viewport: Viewport,
      scope: CoroutineScope,
    ): Editor {
      val host: EditorHost = get()
      val fontLoader: FontLoader = get()

      fontLoader.initFonts()

      val inner = host.createEditor(doc, selection, viewport)
      val editor = Editor(inner, scope)

      editor.on<EditorEvent.StateChanged>(editor.stateChangedHandler)
      editor.on<EditorEvent.FontManifestMissing>(fontLoader.fontManifestMissingHandler)
      editor.on<EditorEvent.FontDataMissing>(fontLoader.fontDataMissingHandler)

      editor.enqueue(Message.System(SystemEvent.Initialize))

      return editor
    }
  }
}
