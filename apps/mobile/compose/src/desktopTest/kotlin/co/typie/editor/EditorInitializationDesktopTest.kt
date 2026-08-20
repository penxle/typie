package co.typie.editor

import co.typie.editor.ffi.ThemeVariant
import java.io.File
import kotlin.test.Test
import kotlin.test.assertFailsWith
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorInitializationDesktopTest {
  @Test
  fun createInitialized_propagates_the_original_initialize_failure() = runTest {
    configureEditorFfiLibrary()
    val failure = IllegalStateException("initialize failed")
    val reported = mutableListOf<Throwable>()

    val thrown =
      assertFailsWith<IllegalStateException> {
        Editor.createInitialized(
          scope = this,
          themeVariant = ThemeVariant.LightWhite,
          dispatcher = UnconfinedTestDispatcher(testScheduler),
          onError = { _, error -> reported += error },
          createInner = { FakeFfiEditor(beforeEnqueueRequest = { throw failure }) },
        )
      }

    assertSame(failure, thrown.cause ?: thrown)
    assertTrue(reported.isEmpty())
  }
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
