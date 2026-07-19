package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue

@Composable
internal fun rememberTextInputState(
  value: String,
  onValueChange: (String) -> Unit,
  enabled: Boolean = true,
  onDismiss: () -> Unit,
): TextInputState {
  val binding = rememberTextInputBindingHandle()
  val state =
    remember(binding) {
      TextInputState(
        binding = binding,
        initialValue = TextFieldValue(value, TextRange(value.length)),
      )
    }

  state.update(value = value, onValueChange = onValueChange, onDismiss = onDismiss)

  DisposableEffect(state, enabled) {
    registerTextInputClient(owner = state.owner, client = if (enabled) state else null)

    onDispose { registerTextInputClient(state.owner, null) }
  }

  return state
}

@Stable
internal class TextInputState
internal constructor(private val binding: TextInputBinding, initialValue: TextFieldValue) :
  TextInputClient {
  private var currentOnValueChange: (String) -> Unit = {}
  private var currentOnDismiss: () -> Unit = {}

  var value by mutableStateOf(initialValue)
    private set

  internal val focusRequester
    get() = binding.focusRequester

  internal val owner: Any
    get() = binding.owner

  fun update(value: String, onValueChange: (String) -> Unit, onDismiss: () -> Unit) {
    currentOnValueChange = onValueChange
    currentOnDismiss = onDismiss
    this.value = syncTextInputValue(currentValue = this.value, text = value)
  }

  fun onValueChange(value: TextFieldValue) {
    applyValueChange(value)
  }

  override val hasActiveComposition: Boolean
    get() = value.composition != null

  override fun requestFocus() {
    binding.requestFocus()
  }

  override fun insertText(text: String): Boolean {
    applyValueChange(insertTextInputValue(currentValue = value, text = text))
    return true
  }

  override fun commitText(text: String) {
    applyValueChange(commitTextInputValue(currentValue = value, text = text))
  }

  override fun setComposingText(text: String) {
    applyValueChange(setComposingTextInputValue(currentValue = value, text = text))
  }

  override fun finishComposition() {
    applyValueChange(finishTextInputComposition(currentValue = value))
  }

  override fun pressKey(key: TextInputKey): Boolean =
    when (key) {
      TextInputKey.Backspace -> {
        applyValueChange(deleteBackwardTextInputValue(currentValue = value))
        true
      }
      TextInputKey.Enter -> false
    }

  override fun dismiss() {
    currentOnDismiss()
  }

  private fun applyValueChange(nextValue: TextFieldValue) {
    val result = resolveTextInputChange(currentValue = value, nextValue = nextValue)
    value = result.nextValue
    if (result.textChanged) {
      currentOnValueChange(result.nextValue.text)
    }
  }
}

internal fun Modifier.textInputFocusable(
  state: TextInputState,
  enabled: Boolean = true,
  onFocusChange: (FocusState) -> Unit = {},
): Modifier =
  textInputFocusable(
    focusRequester = state.focusRequester,
    owner = state.owner,
    enabled = enabled,
    onFocusChange = onFocusChange,
  )

internal data class TextInputChange(val nextValue: TextFieldValue, val textChanged: Boolean)

internal fun resolveTextInputChange(
  currentValue: TextFieldValue,
  nextValue: TextFieldValue,
): TextInputChange =
  TextInputChange(nextValue = nextValue, textChanged = nextValue.text != currentValue.text)

internal fun syncTextInputValue(currentValue: TextFieldValue, text: String): TextFieldValue {
  if (currentValue.text == text) {
    return currentValue
  }

  val selection =
    if (
      currentValue.selection.collapsed && currentValue.selection.end == currentValue.text.length
    ) {
      TextRange(text.length)
    } else {
      clampTextInputRange(currentValue.selection, text.length)
    }
  return TextFieldValue(text = text, selection = selection)
}

internal fun insertTextInputValue(currentValue: TextFieldValue, text: String): TextFieldValue {
  val range = activeTextInputRange(currentValue)
  val cursor = range.start + text.length
  return replaceTextInputRange(
    currentValue = currentValue,
    range = range,
    replacement = text,
    selection = TextRange(cursor),
  )
}

internal fun commitTextInputValue(currentValue: TextFieldValue, text: String): TextFieldValue {
  val range = activeTextInputRange(currentValue)
  return replaceTextInputRange(
    currentValue = currentValue,
    range = range,
    replacement = text,
    selection = TextRange(range.start + text.length),
  )
}

internal fun setComposingTextInputValue(
  currentValue: TextFieldValue,
  text: String,
): TextFieldValue {
  val range = activeTextInputRange(currentValue)
  val compositionEnd = range.start + text.length
  return replaceTextInputRange(
    currentValue = currentValue,
    range = range,
    replacement = text,
    selection = TextRange(compositionEnd),
    composition = TextRange(range.start, compositionEnd),
  )
}

internal fun finishTextInputComposition(currentValue: TextFieldValue): TextFieldValue =
  currentValue.copy(composition = null)

internal fun deleteBackwardTextInputValue(currentValue: TextFieldValue): TextFieldValue {
  val composition = currentValue.composition?.let(::normalizeTextInputRange)
  if (composition != null && composition.start != composition.end) {
    return replaceTextInputRange(
      currentValue = currentValue,
      range = composition,
      replacement = "",
      selection = TextRange(composition.start),
    )
  }

  val selection = normalizeTextInputRange(currentValue.selection)
  if (selection.start != selection.end) {
    return replaceTextInputRange(
      currentValue = currentValue,
      range = selection,
      replacement = "",
      selection = TextRange(selection.start),
    )
  }

  if (selection.start == 0) {
    return currentValue.copy(composition = null)
  }

  val deleteStart = selection.start - 1
  return replaceTextInputRange(
    currentValue = currentValue,
    range = TextRange(deleteStart, selection.start),
    replacement = "",
    selection = TextRange(deleteStart),
  )
}

internal fun activeTextInputRange(currentValue: TextFieldValue): TextRange =
  currentValue.composition?.let(::normalizeTextInputRange)
    ?: normalizeTextInputRange(currentValue.selection)

internal fun replaceTextInputRange(
  currentValue: TextFieldValue,
  range: TextRange,
  replacement: String,
  selection: TextRange,
  composition: TextRange? = null,
): TextFieldValue {
  val normalizedRange =
    clampTextInputRange(normalizeTextInputRange(range), currentValue.text.length)
  val nextText =
    currentValue.text.replaceRange(normalizedRange.start, normalizedRange.end, replacement)

  return TextFieldValue(
    text = nextText,
    selection = clampTextInputRange(selection, nextText.length),
    composition =
      composition?.let { clampTextInputRange(it, nextText.length) }?.takeIf { it.start != it.end },
  )
}

internal fun normalizeTextInputRange(range: TextRange): TextRange =
  if (range.start <= range.end) range else TextRange(range.end, range.start)

internal fun clampTextInputRange(range: TextRange, textLength: Int): TextRange =
  TextRange(start = range.start.coerceIn(0, textLength), end = range.end.coerceIn(0, textLength))
