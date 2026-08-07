package co.typie.platform

import androidx.compose.runtime.Composable

private val UnavailableSoftwareKeyboardPresentationDriverFactory =
  SoftwareKeyboardPresentationDriverFactory {
    null
  }

@Composable
internal actual fun rememberSoftwareKeyboardPresentationDriverFactory():
  SoftwareKeyboardPresentationDriverFactory = UnavailableSoftwareKeyboardPresentationDriverFactory
