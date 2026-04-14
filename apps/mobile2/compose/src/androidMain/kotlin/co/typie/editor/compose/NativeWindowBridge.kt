package co.typie.editor.compose

import android.view.Surface

internal object NativeWindowBridge {
  init {
    System.loadLibrary("editor_ffi")
  }

  external fun fromSurface(surface: Surface): Long

  external fun release(handle: Long)
}
