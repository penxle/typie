package co.typie.ui.component.dialog

interface DialogScope<R> {
  fun resolve(result: R)

  fun dismiss()
}

context(scope: DialogScope<R>)
fun <R> resolve(result: R): Unit = scope.resolve(result)

context(scope: DialogScope<R>)
fun <R> dismiss(): Unit = scope.dismiss()
