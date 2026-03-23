package co.typie.ui.component.bottomsheet

interface BottomSheetScope<T> {
  fun dismiss(result: T)
}

fun BottomSheetScope<Unit>.dismiss() = dismiss(Unit)
