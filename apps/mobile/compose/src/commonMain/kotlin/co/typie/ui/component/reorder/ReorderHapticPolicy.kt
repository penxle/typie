package co.typie.ui.component.reorder

internal class ReorderHapticPolicy(private val minimumIntervalMillis: Long = 50) {
  private var active = false
  private var lastTargetIndex: Int? = null
  private var lastEmissionMillis: Long? = null

  init {
    require(minimumIntervalMillis >= 0)
  }

  fun beginDrag() {
    active = true
    resetHistory()
  }

  fun endDrag() {
    active = false
    resetHistory()
  }

  fun shouldEmit(targetIndex: Int, nowMillis: Long): Boolean {
    if (!active || targetIndex == lastTargetIndex) return false
    lastTargetIndex = targetIndex

    val lastEmission = lastEmissionMillis
    if (lastEmission != null && nowMillis - lastEmission < minimumIntervalMillis) return false

    lastEmissionMillis = nowMillis
    return true
  }

  private fun resetHistory() {
    lastTargetIndex = null
    lastEmissionMillis = null
  }
}
