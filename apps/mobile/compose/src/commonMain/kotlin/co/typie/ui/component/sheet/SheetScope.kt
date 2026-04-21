package co.typie.ui.component.sheet

interface SheetScope<R> {
  fun complete(result: R)

  fun dismiss()
}

context(scope: SheetScope<R>)
fun <R> complete(result: R): Unit = scope.complete(result)

context(scope: SheetScope<R>)
fun <R> dismiss(): Unit = scope.dismiss()
