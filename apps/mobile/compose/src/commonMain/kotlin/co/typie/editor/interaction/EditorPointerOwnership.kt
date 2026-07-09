package co.typie.editor.interaction

internal class EditorPointerOwnership {
  private val pointerIds = mutableSetOf<Long>()

  val hasPointers: Boolean
    get() = pointerIds.isNotEmpty()

  fun acquire(pointerId: Long) {
    pointerIds += pointerId
  }

  fun owns(pointerId: Long): Boolean = pointerId in pointerIds

  fun release(pointerId: Long) {
    pointerIds -= pointerId
  }

  fun reset() {
    pointerIds.clear()
  }
}
