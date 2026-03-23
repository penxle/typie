package co.typie.form

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusRequester
import kotlin.time.Duration

data class DeferredRule<V>(val rule: Rule<V>, val debounce: Duration)

class FieldState<V>(
  initialValue: V,
  internal val rules: List<Rule<V>>,
  internal val deferredRules: List<DeferredRule<V>> = emptyList(),
  internal val validateOn: ValidateOn,
) {
  internal var onValueChanged: (() -> Unit)? = null
  internal var onBlurCallback: (() -> Unit)? = null

  private var _initialValue by mutableStateOf(initialValue)

  var initialValue: V
    get() = _initialValue
    set(newValue) {
      _initialValue = newValue
      value = newValue
      errors = emptyList()
      isTouched = false
    }

  var value: V by mutableStateOf(initialValue)
    internal set

  var errors: List<String> by mutableStateOf(emptyList())
    internal set

  val isDirty: Boolean
    get() = value != _initialValue

  var isTouched: Boolean by mutableStateOf(false)
    private set

  var isValidating: Boolean by mutableStateOf(false)
    internal set

  val focusRequester = FocusRequester()

  internal var form: FormState? = null

  fun setValue(newValue: V) {
    value = newValue
    if (validateOn != ValidateOn.OnChange) {
      errors = emptyList()
    }
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
    value = _initialValue
    errors = emptyList()
    isTouched = false
    isValidating = false
  }

  fun commit() {
    _initialValue = value
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
