package co.typie.ui.input

import android.view.MotionEvent
import androidx.compose.ui.input.pointer.PointerEvent

internal actual val PointerEvent.isTrackpadGesture: Boolean
  get() =
    // ChromeOS also sends these gestures as finger DOWN/MOVE/UP events on Android 13,
    // where Compose does not yet translate them to Pan/Scale pointer events.
    motionEvent?.classification == MotionEvent.CLASSIFICATION_TWO_FINGER_SWIPE ||
      motionEvent?.classification == MotionEvent.CLASSIFICATION_PINCH
