package co.typie.shell

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard
import co.typie.navigation.LocalNavigationForegroundInteractive
import co.typie.navigation.Nav
import co.typie.navigation.Navigator
import co.typie.platform.LocalSoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationDriver
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import co.typie.route.Route
import co.typie.screen.more.feedback.FeedbackForm
import co.typie.screen.settings.updateprofile.UpdateProfileForm
import co.typie.ui.component.TextArea
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

@OptIn(ExperimentalTestApi::class)
class MainShellAutofocusDesktopTest {
  @Test
  fun `main tab motion retains source focus until keyboard hide is accepted`() = runComposeUiTest {
    configureEditorFfiLibrary()
    val keyboardDriver = DeferredMainShellKeyboardDriver()
    val keyboardController = SoftwareKeyboardPresentationController { keyboardDriver }
    var selectTab: ((Tab) -> Unit)? = null
    var currentTab = Tab.Home
    var isBodyMoving = false
    var sourceFocused = false
    var backgroundFocusRequester: FocusRequester? = null

    setContent {
      val sheet = remember { Sheet() }
      val dialog = remember { Dialog() }
      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalSheet provides sheet,
        LocalDialog provides dialog,
        LocalSoftwareKeyboardPresentationController provides keyboardController,
      ) {
        MainShell { route ->
          val tabState = LocalTabState.current
          SideEffect {
            selectTab = tabState.onSelectTab
            currentTab = tabState.currentTab
            isBodyMoving = tabState.isBodyMoving
          }

          when (route) {
            Route.Home -> {
              val focusRequester = remember { FocusRequester() }
              BasicTextField(
                value = "",
                onValueChange = {},
                modifier =
                  Modifier.fillMaxSize().focusRequester(focusRequester).onFocusChanged {
                    sourceFocused = it.isFocused
                  },
              )
              LaunchedEffect(Unit) { focusRequester.requestFocus() }
            }

            Route.Space -> {
              val focusRequester = remember { FocusRequester() }
              SideEffect { backgroundFocusRequester = focusRequester }
              Box(Modifier.fillMaxSize().focusRequester(focusRequester).focusable())
            }

            else -> Unit
          }
        }
      }
    }
    waitUntil { sourceFocused && selectTab != null }

    mainClock.autoAdvance = false
    runOnIdle { requireNotNull(selectTab).invoke(Tab.Space) }
    repeat(8) { mainClock.advanceTimeByFrame() }
    runOnIdle {
      assertTrue(isBodyMoving)
      assertTrue(sourceFocused)
      assertFalse(requireNotNull(backgroundFocusRequester).requestFocus())
      assertTrue(sourceFocused)
    }

    mainClock.autoAdvance = true
    waitUntil(timeoutMillis = 5_000L) { currentTab == Tab.Space && !isBodyMoving }
    waitUntil { keyboardDriver.endpoints == listOf(SoftwareKeyboardPresentationEndpoint.Hidden) }
    runOnIdle {
      assertTrue(sourceFocused)
      assertFalse(requireNotNull(backgroundFocusRequester).requestFocus())
      assertEquals(1f, keyboardDriver.progress.last())
    }

    runOnIdle { keyboardDriver.acceptEndpoint() }
    waitUntil { !sourceFocused }
    runOnIdle { assertTrue(requireNotNull(backgroundFocusRequester).requestFocus()) }
  }

  @Test
  fun `feedback autofocus keeps the software keyboard visible inside main shell`() =
    assertNestedFormAutofocus(Route.Feedback)

  @Test
  fun `update profile autofocus keeps the software keyboard visible inside main shell`() =
    assertNestedFormAutofocus(Route.UpdateProfile)

  private fun assertNestedFormAutofocus(targetRoute: Route) = runComposeUiTest {
    configureEditorFfiLibrary()
    val fixture = MainShellAutofocusFixture(targetRoute)
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
      setContent {
        val sheet = remember { Sheet() }
        val dialog = remember { Dialog() }
        val navigationScope = rememberCoroutineScope()
        SideEffect { fixture.scope = navigationScope }
        CompositionLocalProvider(
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalSheet provides sheet,
          LocalDialog provides dialog,
        ) {
          MainShell { route -> fixture.Content(route) }
        }
      }
      waitUntil { fixture.isReady }

      runOnIdle { fixture.scope.launch { fixture.navigator.navigate(Route.More) } }
      waitUntil(timeoutMillis = 5_000L) {
        fixture.navigator.current == Route.More && !fixture.navigator.isTransitioning
      }

      runOnIdle { fixture.scope.launch { fixture.navigator.navigate(targetRoute) } }
      waitUntil(timeoutMillis = 5_000L) {
        fixture.navigator.current == targetRoute && !fixture.navigator.isTransitioning
      }
      runOnIdle {
        assertEquals(Tab.Home, fixture.currentTab)
        assertEquals(Tab.Home.ordinal.toFloat(), fixture.bodyPosition)
        assertFalse(fixture.isBodyMoving)
        assertFalse(fixture.everBodyMoving, "Autofocus was misclassified as main-tab motion")
        assertTrue(fixture.targetForegroundInteractive)
        assertFalse(
          fixture.targetWasEverForegroundInactive,
          "Target route was briefly made foreground-inactive",
        )
        assertTrue(
          fixture.backgroundTabComposedWhileTargetActive,
          "The regression path did not compose a background tab stack",
        )
      }
      onNode(hasSetTextAction(), useUnmergedTree = true).assertIsFocused()
      runOnIdle {
        assertFalse(
          fixture.backgroundFocusRequester.requestFocus(),
          "An inactive tab stack accepted focus",
        )
      }
      onNode(hasSetTextAction(), useUnmergedTree = true).assertIsFocused()
      waitUntil(timeoutMillis = 5_000L) { DesktopDebugKeyboard.visible }

      val stableSince = System.nanoTime()
      waitUntil(timeoutMillis = 1_000L) {
        System.nanoTime() - stableSince >= KeyboardStabilityNanos
      }

      onNode(hasSetTextAction(), useUnmergedTree = true).assertIsFocused()
      runOnIdle {
        assertTrue(DesktopDebugKeyboard.visible)
        assertEquals(targetRoute, fixture.navigator.current)
        assertEquals(Tab.Home, fixture.currentTab)
        assertFalse(fixture.isBodyMoving)
      }
    } finally {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
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

  private class MainShellAutofocusFixture(private val targetRoute: Route) {
    lateinit var navigator: Navigator
    lateinit var scope: CoroutineScope
    lateinit var backgroundFocusRequester: FocusRequester
    var currentTab = Tab.Home
    var bodyPosition = Tab.Home.ordinal.toFloat()
    var isBodyMoving = false
    var everBodyMoving = false
    var targetForegroundInteractive = false
    var targetWasEverForegroundInactive = false
    var backgroundTabComposedWhileTargetActive = false
    val isReady: Boolean
      get() = ::navigator.isInitialized && ::scope.isInitialized

    @Composable
    fun Content(route: Route) {
      val routeScope = rememberCoroutineScope()
      val tabState = LocalTabState.current
      val nav = Nav.current
      val foregroundInteractive = LocalNavigationForegroundInteractive.current
      SideEffect {
        currentTab = tabState.currentTab
        bodyPosition = tabState.bodyPosition
        isBodyMoving = tabState.isBodyMoving
        everBodyMoving = everBodyMoving || tabState.isBodyMoving
        if (route == targetRoute) {
          targetForegroundInteractive = foregroundInteractive
          targetWasEverForegroundInactive =
            targetWasEverForegroundInactive || !foregroundInteractive
        }
        if (
          isReady && navigator.current == targetRoute && route in setOf(Route.Space, Route.Notes)
        ) {
          backgroundTabComposedWhileTargetActive = true
        }
        if (route == Route.Home) {
          navigator = nav
        }
      }

      Box(Modifier.fillMaxSize().padding(24.dp)) {
        when (route) {
          Route.Feedback -> {
            check(targetRoute == Route.Feedback)
            val form = remember { FeedbackForm(routeScope) }
            TextArea(field = form.content)
          }

          Route.UpdateProfile -> {
            check(targetRoute == Route.UpdateProfile)
            val form = remember { UpdateProfileForm(routeScope) }
            TextField(field = form.name, label = "닉네임")
          }

          Route.Space,
          Route.Notes -> {
            val focusRequester = remember { FocusRequester() }
            SideEffect { backgroundFocusRequester = focusRequester }
            Box(Modifier.focusRequester(focusRequester).focusable())
          }

          else -> Unit
        }
      }
    }
  }

  private companion object {
    const val KeyboardStabilityNanos = 250_000_000L
  }
}

private class DeferredMainShellKeyboardDriver : SoftwareKeyboardPresentationDriver {
  val progress = mutableListOf<Float>()
  val endpoints = mutableListOf<SoftwareKeyboardPresentationEndpoint>()
  private var endpointAcceptance: (() -> Unit)? = null

  override fun updateHiddenProgress(progress: Float) {
    this.progress += progress
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    endpoints += endpoint
    endpointAcceptance = onAccepted
  }

  fun acceptEndpoint() {
    endpointAcceptance?.invoke()
    endpointAcceptance = null
  }

  override fun dispose() = Unit
}
