package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.focus.focusRequester

@Stable
internal class TextInputBinding
internal constructor(internal val focusRequester: FocusRequester, internal val owner: Any) {
  fun requestFocus() {
    focusRequester.requestFocus()
  }
}

@Composable
internal fun rememberTextInputBindingHandle(): TextInputBinding = remember {
  TextInputBinding(focusRequester = FocusRequester(), owner = Any())
}

@Composable
internal fun rememberTextInputBinding(
  enabled: Boolean = true,
  onDismiss: () -> Unit,
): TextInputBinding {
  val binding = rememberTextInputBindingHandle()

  DisposableEffect(binding, enabled, onDismiss) {
    registerTextInputClient(
      owner = binding.owner,
      client =
        if (!enabled) {
          null
        } else {
          object : TextInputClient {
            override fun requestFocus() {
              binding.requestFocus()
            }

            override fun dismiss() {
              onDismiss()
            }
          }
        },
    )

    onDispose { registerTextInputClient(binding.owner, null) }
  }

  return binding
}

internal fun Modifier.textInputFocusable(
  focusRequester: FocusRequester,
  owner: Any,
  enabled: Boolean = true,
  onFocusChange: (FocusState) -> Unit = {},
): Modifier =
  focusRequester(focusRequester)
    .textInputFocusChanged(owner = owner, enabled = enabled, onFocusChange = onFocusChange)

internal fun Modifier.textInputFocusable(
  binding: TextInputBinding,
  enabled: Boolean = true,
  onFocusChange: (FocusState) -> Unit = {},
): Modifier =
  textInputFocusable(
    focusRequester = binding.focusRequester,
    owner = binding.owner,
    enabled = enabled,
    onFocusChange = onFocusChange,
  )
