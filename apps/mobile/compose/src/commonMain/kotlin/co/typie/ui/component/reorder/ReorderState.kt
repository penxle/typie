package co.typie.ui.component.reorder

data class ReorderDrop<K : Any>(
  val movedKey: K,
  val fromIndex: Int,
  val toIndex: Int,
  val orderedKeys: List<K>,
)

private data class ActiveReorder<K : Any>(
  val key: K,
  val startIndex: Int,
  val startKeys: List<K>,
  val restoreLatestInputOnCancel: Boolean,
)

private data class ReorderSettling<K : Any>(val key: K, val initialOffsetY: Float)

internal class ReorderState<K : Any>(initialKeys: List<K> = emptyList()) {
  private var _inputKeys = initialKeys.toList()
  private var _keys = initialKeys.toList()
  private var activeDrag: ActiveReorder<K>? = null
  private var settling: ReorderSettling<K>? = null
  private var _orderRevision = 0L

  internal var onChanged: (() -> Unit)? = null

  var inputKeys: List<K>
    get() = _inputKeys
    set(value) {
      val nextKeys = value.toList()
      val previousInputKeys = _inputKeys
      if (previousInputKeys == nextKeys) return
      _inputKeys = nextKeys

      when {
        previousInputKeys.toSet() != nextKeys.toSet() -> resetOrder(nextKeys)
        activeDrag == null && _keys == previousInputKeys -> setKeys(nextKeys)
        else -> notifyChanged()
      }
    }

  val keys: List<K>
    get() = _keys

  val layoutKeys: List<K>
    get() = activeDrag?.startKeys ?: _keys

  val draggingKey: K?
    get() = activeDrag?.key

  val isDragging: Boolean
    get() = activeDrag != null

  val orderRevision: Long
    get() = _orderRevision

  fun isDragging(key: K): Boolean = activeDrag?.key == key

  fun isSettling(key: K): Boolean = settling?.key == key

  fun beginDrag(key: K): Boolean {
    if (activeDrag != null) return false
    val startIndex = _keys.indexOf(key)
    if (startIndex == -1) return false

    settling = null
    activeDrag =
      ActiveReorder(
        key = key,
        startIndex = startIndex,
        startKeys = _keys,
        restoreLatestInputOnCancel = _keys == inputKeys,
      )
    notifyChanged()
    return true
  }

  fun moveDraggedTo(targetIndex: Int): Boolean {
    val drag = activeDrag ?: return false
    val currentIndex = _keys.indexOf(drag.key)
    if (currentIndex == -1) return false
    val resolvedTarget = targetIndex.coerceIn(0, _keys.lastIndex)
    if (currentIndex == resolvedTarget) return false

    val reordered = _keys.toMutableList()
    reordered.removeAt(currentIndex)
    reordered.add(resolvedTarget, drag.key)
    setKeys(reordered)
    return true
  }

  fun endDrag(releaseOffsetY: Float): ReorderDrop<K>? {
    val drag = activeDrag ?: return null
    val finalKeys = _keys
    activeDrag = null
    settling =
      releaseOffsetY
        .takeUnless { it == 0f }
        ?.let { ReorderSettling(key = drag.key, initialOffsetY = it) }
    notifyChanged()

    if (finalKeys == drag.startKeys) return null
    return ReorderDrop(
      movedKey = drag.key,
      fromIndex = drag.startIndex,
      toIndex = finalKeys.indexOf(drag.key),
      orderedKeys = finalKeys,
    )
  }

  fun cancelDrag() {
    val drag = activeDrag
    resetOrder(
      if (drag?.restoreLatestInputOnCancel == false) {
        drag.startKeys
      } else {
        inputKeys
      }
    )
  }

  fun resetOrder(keys: List<K>) {
    activeDrag = null
    settling = null
    setKeys(keys.toList(), notifyWhenUnchanged = true)
  }

  fun settlingOffsetY(key: K): Float? = settling?.takeIf { it.key == key }?.initialOffsetY

  fun clearSettling(key: K) {
    if (settling?.key != key) return
    settling = null
    notifyChanged()
  }

  private fun setKeys(keys: List<K>, notifyWhenUnchanged: Boolean = false) {
    if (_keys == keys) {
      if (notifyWhenUnchanged) notifyChanged()
      return
    }
    _keys = keys
    _orderRevision += 1
    notifyChanged()
  }

  private fun notifyChanged() {
    onChanged?.invoke()
  }
}
