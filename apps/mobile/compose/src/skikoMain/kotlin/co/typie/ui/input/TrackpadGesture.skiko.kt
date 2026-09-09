package co.typie.ui.input

import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventType

internal actual val PointerEvent.isTrackpadGesture: Boolean
  get() =
    type == PointerEventType.PanStart ||
      type == PointerEventType.PanMove ||
      type == PointerEventType.ScaleStart ||
      type == PointerEventType.ScaleChange
