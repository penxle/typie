package co.typie.form

class FormValidationScope
internal constructor(private val fieldErrors: Map<FieldState<*>, List<String>>) {
  internal val collectedFieldErrors = mutableMapOf<FieldState<*>, MutableList<String>>()
  internal val collectedFormErrors = mutableListOf<String>()

  fun check(field: FieldState<*>, condition: Boolean, message: () -> String) {
    if (fieldErrors[field]?.isNotEmpty() == true) return
    if (!condition) {
      collectedFieldErrors.getOrPut(field) { mutableListOf() }.add(message())
    }
  }

  fun check(condition: Boolean, message: () -> String) {
    if (!condition) collectedFormErrors.add(message())
  }
}
