package co.typie.form

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope

@Composable
inline fun <T : FormState> rememberFormState(crossinline factory: () -> T): T {
  val scope = rememberCoroutineScope()
  val form = remember { factory() }
  LaunchedEffect(form) {
    form.setValidationScope(scope)
  }
  return form
}
