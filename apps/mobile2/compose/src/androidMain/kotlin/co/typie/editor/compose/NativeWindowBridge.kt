package co.typie.editor.compose

internal object NativeWindowBridge {
  init {
    System.loadLibrary("editor_ffi")
  }

  external fun fromSurface(surface: android.view.Surface): Long

  external fun release(handle: Long)
}
