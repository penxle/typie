package co.typie.form

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope

@Composable
inline fun <T : FormState> rememberFormState(
  vararg keys: Any?,
  crossinline factory: () -> T,
): T {
  val scope = rememberCoroutineScope()
  val form = remember(*keys) { factory() }
  LaunchedEffect(form) {
    form.setValidationScope(scope)
  }
  return form
}
