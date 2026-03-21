package co.typie.form

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.time.Duration

data class DeferredRule<V>(val rule: Rule<V>, val debounce: Duration)

class FieldState<V>(
  private val initialValue: V,
  internal val rules: List<Rule<V>>,
  internal val deferredRules: List<DeferredRule<V>> = emptyList(),
  internal val validateOn: ValidateOn,
) {
  internal var onValueChanged: (() -> Unit)? = null
  internal var onBlurCallback: (() -> Unit)? = null

  var value: V by mutableStateOf(initialValue)
    internal set

  var errors: List<String> by mutableStateOf(emptyList())
    internal set

  var isDirty: Boolean by mutableStateOf(false)
    private set

  var isTouched: Boolean by mutableStateOf(false)
    private set

  var isValidating: Boolean by mutableStateOf(false)
    internal set

  fun setValue(newValue: V) {
    value = newValue
    if (validateOn != ValidateOn.OnChange) {
      errors = emptyList()
    }
    isDirty = newValue != initialValue
    onValueChanged?.invoke()
  }

  fun setErrors(newErrors: List<String>) {
    errors = newErrors
  }

  fun onBlur() {
    isTouched = true
    onBlurCallback?.invoke()
  }

  fun reset() {
    value = initialValue
    errors = emptyList()
    isDirty = false
    isTouched = false
    isValidating = false
  }

  internal suspend fun validate(): List<String> {
    val result = mutableListOf<String>()
    for (rule in rules + deferredRules.map { it.rule }) {
      rule.validate(value)?.let { result.add(it) }
    }
    return result
  }

  operator fun component1(): V = value
  operator fun component2(): (V) -> Unit = { setValue(it) }
  operator fun component3(): List<String> = errors
}
