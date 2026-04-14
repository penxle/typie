package co.typie.form

import androidx.compose.ui.text.input.ImeAction
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class FieldConfig<V> {
  internal val taggedRules = mutableListOf<Pair<ValidateOn?, Rule<V>>>()
  internal val deferredRules = mutableListOf<DeferredRule<V>>()
  internal var validateOn: ValidateOn? = null
  var focusable: Boolean = true
  private var blockValidateOn: ValidateOn? = null

  internal fun addRule(rule: Rule<V>) {
    taggedRules.add(blockValidateOn to rule)
  }

  fun rules(vararg rules: Rule<V>) {
    for (r in rules) addRule(r)
  }

  fun validateOn(timing: ValidateOn) {
    this.validateOn = timing
  }

  fun validateOn(timing: ValidateOn, block: FieldConfig<V>.() -> Unit) {
    val previous = blockValidateOn
    blockValidateOn = timing
    block()
    blockValidateOn = previous
  }

  fun required(message: String = "필수 항목입니다") {
    addRule(co.typie.form.required(message))
  }

  fun rule(validate: suspend (V) -> String?) {
    addRule(co.typie.form.rule(validate))
  }

  fun defer(debounce: Duration = 300.milliseconds, validate: suspend (V) -> String?) {
    deferredRules.add(DeferredRule(Rule { validate(it) }, debounce))
  }
}

// String-specific convenience methods
fun FieldConfig<String>.email(message: String = "올바른 이메일 형식을 입력해주세요") {
  addRule(co.typie.form.email(message))
}

fun FieldConfig<String>.minLength(min: Int, message: String = "${min}자 이상 입력해주세요") {
  addRule(co.typie.form.minLength(min, message))
}

fun FieldConfig<String>.maxLength(max: Int, message: String = "${max}자 이하로 입력해주세요") {
  addRule(co.typie.form.maxLength(max, message))
}

fun FieldConfig<String>.pattern(regex: Regex, message: String = "올바른 형식을 입력해주세요") {
  addRule(co.typie.form.pattern(regex, message))
}

// Comparable-specific convenience methods
fun <V : Comparable<V>> FieldConfig<V>.min(min: V, message: String = "최솟값은 ${min}입니다") {
  addRule(co.typie.form.min(min, message))
}

fun <V : Comparable<V>> FieldConfig<V>.max(max: V, message: String = "최댓값은 ${max}입니다") {
  addRule(co.typie.form.max(max, message))
}

open class FormState(
  private val scope: CoroutineScope,
  private val defaultValidateOn: ValidateOn = ValidateOn.Submit,
) {
  private val registeredFields = mutableListOf<FieldState<*>>()
  private val debounceJobs = mutableMapOf<FieldState<*>, Job>()

  protected fun <V> field(initialValue: V, config: FieldConfig<V>.() -> Unit = {}): FieldState<V> {
    val fieldConfig = FieldConfig<V>().apply(config)
    val fieldDefault = fieldConfig.validateOn ?: defaultValidateOn
    val rulesByTiming =
      fieldConfig.taggedRules.groupBy(
        keySelector = { (timing, _) -> timing ?: fieldDefault },
        valueTransform = { (_, rule) -> rule },
      )
    val fieldState =
      FieldState(
        initialValue = initialValue,
        rulesByTiming = rulesByTiming,
        deferredRules = fieldConfig.deferredRules,
      )
    fieldState.onValueChanged = { onFieldValueChanged(fieldState) }
    fieldState.onBlurCallback = {
      if (ValidateOn.Blur in fieldState.rulesByTiming) {
        scope.launch {
          fieldState.isValidating = true
          fieldState.errors = fieldState.validateForEvent(ValidateOn.Blur)
          fieldState.isValidating = false
        }
      }
    }
    fieldState.focusable = fieldConfig.focusable
    fieldState.form = this
    registeredFields.add(fieldState)
    return fieldState
  }

  val isDirty: Boolean
    get() = registeredFields.any { it.isDirty }

  val isValid: Boolean
    get() = registeredFields.all { it.errors.isEmpty() }

  val errors: Map<FieldState<*>, List<String>>
    get() = registeredFields.filter { it.errors.isNotEmpty() }.associateWith { it.errors }

  val errorMessage
    get() = errors.values.flatten().firstOrNull()

  suspend fun validateAll(): Boolean {
    var allValid = true
    for (field in registeredFields) {
      field.isValidating = true
      val errors = field.validate()
      field.errors = errors
      field.isValidating = false
      if (errors.isNotEmpty()) allValid = false
    }
    return allValid
  }

  suspend fun validate(): Boolean {
    val valid = validateAll()
    if (!valid) focusFirstError()
    return valid
  }

  fun reset() {
    rollback()
  }

  fun rollback() {
    for (field in registeredFields) {
      field.rollback()
    }

    debounceJobs.values.forEach { it.cancel() }
    debounceJobs.clear()
  }

  fun commit() {
    for (field in registeredFields) {
      field.commit()
    }
  }

  private val focusChain: List<FieldState<*>>
    get() = registeredFields.filter { it.focusable }

  fun isFirstField(field: FieldState<*>): Boolean = focusChain.firstOrNull() == field

  fun isLastField(field: FieldState<*>): Boolean = focusChain.lastOrNull() == field

  fun focusNext(field: FieldState<*>) {
    val chain = focusChain
    val index = chain.indexOf(field)
    if (index < 0 || index == chain.lastIndex) return
    chain[index + 1].focusRequester.requestFocus()
  }

  fun focusPrevious(field: FieldState<*>) {
    val chain = focusChain
    val index = chain.indexOf(field)
    if (index <= 0) return
    chain[index - 1].focusRequester.requestFocus()
  }

  fun imeActionFor(field: FieldState<*>): ImeAction =
    if (isLastField(field)) ImeAction.Done else ImeAction.Next

  fun focusFirstError() {
    focusChain.firstOrNull { it.errors.isNotEmpty() }?.focusRequester?.requestFocus()
  }

  private fun onFieldValueChanged(field: FieldState<*>) {
    val hasOnChangeRules = ValidateOn.Change in field.rulesByTiming
    val hasDeferredRules = field.deferredRules.isNotEmpty()
    if (!hasOnChangeRules && !hasDeferredRules) return

    debounceJobs[field]?.cancel()

    debounceJobs[field] = scope.launch {
      // Phase 1: sync OnChange rules (no delay)
      if (hasOnChangeRules) {
        field.errors = field.validateForEvent(ValidateOn.Change)
      }

      // Phase 2: deferred rules (with delay)
      if (hasDeferredRules) {
        val debounce = field.deferredRules.maxOf { it.debounce }
        delay(debounce)
        field.isValidating = true
        field.errors = field.validateOnChangeWithDeferred()
        field.isValidating = false
      }
    }
  }
}
