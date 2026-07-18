package co.typie.editor.interaction

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers

internal expect fun Modifier.editorPlatformIndirectScale(
  bridge: EditorPlatformIndirectScaleBridge,
  enabled: Boolean,
  density: Float,
): Modifier

internal expect fun PointerKeyboardModifiers.isEditorIndirectZoomModifierPressed(): Boolean

internal class EditorPlatformIndirectScaleBridge {
  private var owner: EditorPlatformIndirectScaleOwner? = null

  fun attach(owner: EditorPlatformIndirectScaleOwner) {
    this.owner = owner
  }

  fun detach(owner: EditorPlatformIndirectScaleOwner) {
    if (this.owner === owner) {
      this.owner = null
    }
  }

  fun begin(): Boolean = owner?.beginIndirectScale() == true

  fun update(focalInRootPx: Offset, scaleFactor: Float): Boolean =
    owner?.updateIndirectScale(focalInRootPx, scaleFactor) == true

  fun end() {
    owner?.endIndirectScale()
  }
}

internal interface EditorPlatformIndirectScaleOwner {
  fun beginIndirectScale(): Boolean

  fun updateIndirectScale(focalInRootPx: Offset, scaleFactor: Float): Boolean

  fun endIndirectScale()
}
