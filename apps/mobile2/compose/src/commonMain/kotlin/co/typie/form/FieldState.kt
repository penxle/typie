package co.typie.form

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.focus.FocusRequester
import kotlin.time.Duration

data class DeferredRule<V>(val rule: Rule<V>, val debounce: Duration)

class FieldState<V>(
  initialValue: V,
  internal val rulesByTiming: Map<ValidateOn, List<Rule<V>>>,
  internal val deferredRules: List<DeferredRule<V>> = emptyList(),
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

  internal var focusable: Boolean = true
  internal var form: FormState? = null

  fun setValue(newValue: V) {
    value = newValue
    if (ValidateOn.Change !in rulesByTiming && deferredRules.isEmpty()) {
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
    rollback()
  }

  fun rollback() {
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
    for (rules in rulesByTiming.values) {
      for (rule in rules) {
        rule.validate(value)?.let { result.add(it) }
      }
    }
    for (deferred in deferredRules) {
      deferred.rule.validate(value)?.let { result.add(it) }
    }
    return result
  }

  internal suspend fun validateForEvent(event: ValidateOn): List<String> {
    val rules: List<Rule<V>> = when (event) {
      ValidateOn.Change -> rulesByTiming[ValidateOn.Change].orEmpty()
      ValidateOn.Blur -> rulesByTiming[ValidateOn.Change].orEmpty() + rulesByTiming[ValidateOn.Blur].orEmpty()
      ValidateOn.Submit -> rulesByTiming.values.flatten()
    }
    return rules.mapNotNull { it.validate(value) }
  }

  internal suspend fun validateOnChangeWithDeferred(): List<String> {
    val rules = rulesByTiming[ValidateOn.Change].orEmpty() + deferredRules.map { it.rule }
    return rules.mapNotNull { it.validate(value) }
  }

  operator fun component1(): V = value
  operator fun component2(): (V) -> Unit = { setValue(it) }
  operator fun component3(): List<String> = errors
}
