package co.typie.screen.home.search

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.dev.ProvideDesktopDebugKeyboardPresentation
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import java.io.File
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class SearchScreenDesktopTest {
  @Test
  fun `search input autofocus is not repeated when the same route entry is reexposed`() =
    runComposeUiTest {
      configureEditorFfiLibrary()
      val owner =
        object : ViewModelStoreOwner {
          override val viewModelStore = ViewModelStore()
        }
      var visible by mutableStateOf(true)

      setContent {
        val dialog = remember { Dialog() }
        val sheet = remember { Sheet() }
        ProvideDesktopDebugKeyboardPresentation {
          CompositionLocalProvider(
            LocalViewModelStoreOwner provides owner,
            LocalThemeMode provides ResolvedThemeMode.Light,
            LocalDialog provides dialog,
            LocalSheet provides sheet,
          ) {
            if (visible) SearchScreen()
          }
        }
      }
      waitForIdle()

      onNode(hasSetTextAction(), useUnmergedTree = true).assertIsFocused()

      runOnIdle { visible = false }
      waitForIdle()
      runOnIdle { visible = true }
      waitForIdle()

      onNode(hasSetTextAction(), useUnmergedTree = true).assertIsNotFocused()
      owner.viewModelStore.clear()
    }

  private fun configureEditorFfiLibrary() {
    if (System.getProperty("jna.library.path") != null) return

    val repository =
      generateSequence(File(System.getProperty("user.dir"))) { it.parentFile }
        .firstOrNull { File(it, "Cargo.toml").isFile } ?: error("Typie repository root not found")
    val host =
      when (System.getProperty("os.arch")) {
        "aarch64" -> "aarch64-apple-darwin"
        "x86_64" -> "x86_64-apple-darwin"
        else -> error("Unsupported desktop test architecture: ${System.getProperty("os.arch")}")
      }
    val directory = File(repository, "target/$host/release-uniffi")
    check(File(directory, "libeditor_ffi.dylib").isFile) {
      "Desktop editor FFI is not built; run `just -f crates/editor-ffi/justfile desktop`"
    }
    System.setProperty("jna.library.path", directory.absolutePath)
  }
}
