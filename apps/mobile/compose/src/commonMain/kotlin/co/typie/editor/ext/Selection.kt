package co.typie.editor.ext

import co.typie.editor.ffi.Selection
import kotlin.math.abs

internal fun Selection?.isCollapsed(): Boolean = this == null || anchor == head

internal fun Selection?.isSingleSlotRange(): Boolean =
  this != null && anchor.node == head.node && abs(anchor.offset - head.offset) == 1
