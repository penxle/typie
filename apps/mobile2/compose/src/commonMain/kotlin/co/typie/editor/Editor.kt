package co.typie.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
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
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
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

  internal val focusRequester = FocusRequester()
  internal var focusManager: FocusManager? = null
  internal val pageOffsets = mutableStateMapOf<Int, Offset>()

  fun focus() = focusRequester.requestFocus()
  fun blur() { focusManager?.clearFocus() }

  fun localToGlobal(page: Int, x: Float, y: Float): Offset? {
    val offset = pageOffsets[page] ?: return null
    return Offset(offset.x + x, offset.y + y)
  }

  fun globalToLocal(x: Float, y: Float): PagePoint? {
    val sizes = pageSizes
    if (sizes.isEmpty()) return null

    var lo = 0
    var hi = sizes.lastIndex

    while (lo < hi) {
      val mid = (lo + hi) ushr 1
      val midOffset = pageOffsets[mid] ?: return null
      if (midOffset.y + sizes[mid].height <= y) lo = mid + 1
      else hi = mid
    }

    val loOffset = pageOffsets[lo] ?: return null
    var localY = y - loOffset.y

    if (localY < 0 && lo > 0) {
      val prevOffset = pageOffsets[lo - 1] ?: return null
      val prevBottom = prevOffset.y + sizes[lo - 1].height
      if (y < (prevBottom + loOffset.y) / 2) {
        lo--
        localY = sizes[lo].height
      } else {
        localY = 0f
      }
    }

    val finalOffset = pageOffsets[lo] ?: return null
    val size = sizes[lo]
    val localX = (x - finalOffset.x).coerceIn(0f, size.width)
    localY = localY.coerceIn(0f, size.height)
    return PagePoint(lo, localX, localY)
  }

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
    if (!batching && !queued) {
      queued = true
      scope.launch(Dispatchers.Main) { tick() }
    }
  }

  private var batching = false

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
