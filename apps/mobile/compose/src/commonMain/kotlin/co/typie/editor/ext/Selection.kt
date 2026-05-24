package co.typie.editor.ext

import co.typie.editor.ffi.Selection

internal fun Selection?.isCollapsed(): Boolean = this == null || anchor == head
