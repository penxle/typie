@file:OptIn(ExperimentalComposeUiApi::class)

package co.typie

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.uikit.OnFocusBehavior
import androidx.compose.ui.window.ComposeUIViewController

fun MainViewController() = ComposeUIViewController(
  configure = {
    onFocusBehavior = OnFocusBehavior.DoNothing
    parallelRendering = true
  },
) { App() }
