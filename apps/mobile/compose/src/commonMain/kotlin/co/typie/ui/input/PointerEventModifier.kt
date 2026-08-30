package co.typie.ui.input

import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.SuspendingPointerInputModifierNode
import androidx.compose.ui.node.DelegatingNode
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.platform.InspectorInfo

// Adapted from Compose Multiplatform's skiko-only Modifier.onPointerEvent under Apache-2.0, which
// is unavailable on Android.
// Copyright 2022 The Android Open Source Project.
internal fun Modifier.onPointerEvent(
  eventType: PointerEventType,
  pass: PointerEventPass = PointerEventPass.Main,
  onEvent: AwaitPointerEventScope.(event: PointerEvent) -> Unit,
): Modifier = this then OnPointerEventElement(eventType = eventType, pass = pass, onEvent = onEvent)

private class OnPointerEventElement(
  private val eventType: PointerEventType,
  private val pass: PointerEventPass,
  private val onEvent: AwaitPointerEventScope.(event: PointerEvent) -> Unit,
) : ModifierNodeElement<OnPointerEventNode>() {
  override fun create() = OnPointerEventNode(eventType = eventType, pass = pass, onEvent = onEvent)

  override fun update(node: OnPointerEventNode) =
    node.update(eventType = eventType, pass = pass, onEvent = onEvent)

  override fun InspectorInfo.inspectableProperties() {
    name = "onPointerEvent"
    properties["eventType"] = eventType
    properties["pass"] = pass
    properties["onEvent"] = onEvent
  }

  override fun hashCode(): Int {
    var result = eventType.hashCode()
    result = 31 * result + pass.hashCode()
    return 31 * result + onEvent.hashCode()
  }

  override fun equals(other: Any?): Boolean {
    if (this === other) return true
    if (other !is OnPointerEventElement) return false
    return eventType == other.eventType && pass == other.pass && onEvent === other.onEvent
  }
}

private class OnPointerEventNode(
  private var eventType: PointerEventType,
  private var pass: PointerEventPass,
  private var onEvent: AwaitPointerEventScope.(event: PointerEvent) -> Unit,
) : DelegatingNode() {
  private val pointerInputNode =
    delegate(
      SuspendingPointerInputModifierNode {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent(pass)
            if (event.type == eventType) {
              onEvent(event)
            }
          }
        }
      }
    )

  fun update(
    eventType: PointerEventType,
    pass: PointerEventPass,
    onEvent: AwaitPointerEventScope.(event: PointerEvent) -> Unit,
  ) {
    this.eventType = eventType
    this.onEvent = onEvent

    if (this.pass != pass) {
      this.pass = pass
      pointerInputNode.resetPointerInputHandler()
    }
  }
}
