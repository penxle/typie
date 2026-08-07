package co.typie.dev

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ext.TextInputClient
import co.typie.ext.ime
import co.typie.platform.LocalSoftwareKeyboardPresentationController
import co.typie.platform.ProvideSoftwareKeyboardPresentation
import co.typie.platform.SoftwareKeyboardInteractionResolution
import co.typie.platform.SoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class DesktopDebugKeyboardPresentationDesktopTest {
  @Test
  fun ordinaryShowAndHideKeepTheSurfaceAndImeInsetTogether() = runComposeUiTest {
    val owner = Any()
    val client = RecordingTextInputClient()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var imeBottomPx = 0f
    var shownHeightPx = 0f

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
      }
      setContent {
        CompositionLocalProvider(LocalAppColors provides LightColors) {
          ProvideSoftwareKeyboardPresentation {
            ProvideDesktopDebugKeyboardPresentation {
              val density = LocalDensity.current
              val imeBottom = WindowInsets.ime.getBottom(density).toFloat()
              SideEffect {
                imeBottomPx = imeBottom
                shownHeightPx = with(density) { DesktopDebugKeyboard.height.toPx() }
              }
              Box(Modifier.size(width = 400.dp, height = 700.dp).testTag(ROOT_TAG)) {
                DesktopDebugKeyboard.Overlay(
                  Modifier.align(Alignment.BottomStart)
                    .padding(bottom = DESKTOP_CHROME_BOTTOM_OFFSET)
                    .testTag(KEYBOARD_TAG)
                )
              }
            }
          }
        }
      }

      onNodeWithTag(KEYBOARD_TAG).assertDoesNotExist()
      assertEquals(0f, imeBottomPx)

      mainClock.autoAdvance = false
      runOnIdle { DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true) }
      mainClock.advanceTimeBy(110L)
      waitForIdle()

      assertSurfaceMatchesImeInset(imeBottomPx)
      val intermediateHeight = onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height
      assertTrue(intermediateHeight in 1f..<shownHeightPx)

      mainClock.advanceTimeBy(160L)
      waitForIdle()

      assertSurfaceMatchesImeInset(imeBottomPx)
      assertEquals(
        shownHeightPx,
        onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot.height,
        absoluteTolerance = POSITION_TOLERANCE_PX,
      )

      runOnIdle { DesktopDebugKeyboard.hideKeyboardSurface() }
      mainClock.advanceTimeBy(110L)
      waitForIdle()

      assertSurfaceMatchesImeInset(imeBottomPx)
      mainClock.advanceTimeBy(160L)
      waitForIdle()

      onNodeWithTag(KEYBOARD_TAG).assertDoesNotExist()
      assertEquals(0f, imeBottomPx)
    } finally {
      mainClock.autoAdvance = true
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun interactiveSessionCanStartOnlyAfterOrdinaryShowReachesItsEndpoint() = runComposeUiTest {
    val owner = Any()
    val client = RecordingTextInputClient()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
      }
      setContent {
        ProvideSoftwareKeyboardPresentation {
          ProvideDesktopDebugKeyboardPresentation {
            val currentController = LocalSoftwareKeyboardPresentationController.current
            SideEffect { controller = currentController }
            Box(Modifier.fillMaxSize())
          }
        }
      }
      waitForIdle()

      mainClock.autoAdvance = false
      runOnIdle { DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true) }
      mainClock.advanceTimeBy(110L)
      waitForIdle()

      runOnIdle { assertNull(controller.acquire()) }

      mainClock.advanceTimeBy(160L)
      waitForIdle()

      runOnIdle { requireNotNull(controller.acquire()).dispose() }
    } finally {
      mainClock.autoAdvance = true
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun interactiveSessionRequiresACurrentFocusedClient() = runComposeUiTest {
    val owner = Any()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true)
      }
      setContent {
        ProvideSoftwareKeyboardPresentation {
          ProvideDesktopDebugKeyboardPresentation {
            val currentController = LocalSoftwareKeyboardPresentationController.current
            SideEffect { controller = currentController }
            Box(Modifier.fillMaxSize())
          }
        }
      }
      waitForIdle()

      runOnIdle { assertNull(controller.acquire()) }
    } finally {
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun focusLossRevokesInteractiveAdmissionBeforeTheDelayedHide() = runComposeUiTest {
    val owner = Any()
    val client = RecordingTextInputClient()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true)
      }
      setContent {
        ProvideSoftwareKeyboardPresentation {
          ProvideDesktopDebugKeyboardPresentation {
            val currentController = LocalSoftwareKeyboardPresentationController.current
            SideEffect { controller = currentController }
            Box(Modifier.fillMaxSize())
          }
        }
      }
      waitForIdle()

      runOnIdle { DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false) }

      runOnIdle { assertNull(controller.acquire()) }
    } finally {
      runOnIdle {
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun interactiveProgressUsesOneHeightAndRetainsInputUntilHiddenCompletion() = runComposeUiTest {
    val owner = Any()
    val client = RecordingTextInputClient()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController
    var imeBottomPx = 0f
    var shownHeightPx = 0f
    var bottomOffsetPx = 0f

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true)
      }
      setContent {
        CompositionLocalProvider(LocalAppColors provides LightColors) {
          ProvideSoftwareKeyboardPresentation {
            ProvideDesktopDebugKeyboardPresentation {
              val density = LocalDensity.current
              val currentController = LocalSoftwareKeyboardPresentationController.current
              val imeBottom = WindowInsets.ime.getBottom(density).toFloat()
              SideEffect {
                controller = currentController
                imeBottomPx = imeBottom
                shownHeightPx = with(density) { DesktopDebugKeyboard.height.toPx() }
                bottomOffsetPx = with(density) { DESKTOP_CHROME_BOTTOM_OFFSET.toPx() }
              }
              Box(Modifier.size(width = 400.dp, height = 700.dp).testTag(ROOT_TAG)) {
                DesktopDebugKeyboard.Overlay(
                  Modifier.align(Alignment.BottomStart)
                    .padding(bottom = DESKTOP_CHROME_BOTTOM_OFFSET)
                    .testTag(KEYBOARD_TAG)
                )
              }
            }
          }
        }
      }
      waitForIdle()
      assertSurfaceMatchesImeInset(imeBottomPx)

      mainClock.autoAdvance = false
      val session = runOnIdle { requireNotNull(controller.acquire()) }
      runOnIdle { session.updateHiddenProgress(0.5f) }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()
      waitForIdle()

      assertSurfaceMatchesImeInset(imeBottomPx)
      assertEquals(
        shownHeightPx * 0.5f + bottomOffsetPx,
        imeBottomPx,
        absoluteTolerance = POSITION_TOLERANCE_PX,
      )

      runOnIdle { session.updateHiddenProgress(1f) }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()
      waitForIdle()

      onNodeWithTag(KEYBOARD_TAG).assertDoesNotExist()
      assertEquals(0f, imeBottomPx)
      runOnIdle {
        assertTrue(DesktopDebugKeyboard.visible)
        assertTrue(controller.interactionState.unresolved)
        assertNull(controller.interactionState.lastResolution)
      }

      runOnIdle { session.finish(SoftwareKeyboardPresentationEndpoint.Hidden) }
      mainClock.advanceTimeByFrame()
      waitForIdle()

      onNodeWithTag(KEYBOARD_TAG).assertDoesNotExist()
      assertEquals(0f, imeBottomPx)
      runOnIdle {
        assertFalse(DesktopDebugKeyboard.visible)
        assertFalse(controller.interactionState.unresolved)
        assertEquals(
          SoftwareKeyboardInteractionResolution.Hidden,
          controller.interactionState.lastResolution,
        )
        DesktopDebugKeyboard.showKeyboard()
        assertTrue(DesktopDebugKeyboard.visible, "The focused registered client was released early")
      }
    } finally {
      mainClock.autoAdvance = true
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun committedHideContinuesDownwardAfterInputSessionEnds() = runComposeUiTest {
    val owner = Any()
    val client = RecordingTextInputClient()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController
    var imeBottomPx = 0f

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true)
      }
      setContent {
        ProvideSoftwareKeyboardPresentation {
          ProvideDesktopDebugKeyboardPresentation {
            val density = LocalDensity.current
            val currentController = LocalSoftwareKeyboardPresentationController.current
            val imeBottom = WindowInsets.ime.getBottom(density).toFloat()
            SideEffect {
              controller = currentController
              imeBottomPx = imeBottom
            }
            Box(Modifier.fillMaxSize())
          }
        }
      }
      waitForIdle()

      mainClock.autoAdvance = false
      val session = runOnIdle { requireNotNull(controller.acquire()) }
      runOnIdle { session.updateHiddenProgress(0.8f) }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()
      waitForIdle()
      val insetBeforeCommittedHide = imeBottomPx

      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        session.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
      }
      mainClock.advanceTimeBy(60L)
      waitForIdle()

      assertTrue(
        imeBottomPx <= insetBeforeCommittedHide + POSITION_TOLERANCE_PX,
        "Committed hide must continue downward after the input session ends",
      )

      mainClock.advanceTimeBy(220L)
      waitForIdle()

      assertEquals(0f, imeBottomPx)
      runOnIdle {
        assertFalse(DesktopDebugKeyboard.visible)
        assertEquals(
          SoftwareKeyboardInteractionResolution.Hidden,
          controller.interactionState.lastResolution,
        )
      }
    } finally {
      mainClock.autoAdvance = true
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun shownCompletionHasNoGeometryJump() = runComposeUiTest {
    val owner = Any()
    val client = RecordingTextInputClient()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController
    var imeBottomPx = 0f

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
        DesktopDebugKeyboard.registerClient(owner, client)
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = true)
      }
      setContent {
        CompositionLocalProvider(LocalAppColors provides LightColors) {
          ProvideSoftwareKeyboardPresentation {
            ProvideDesktopDebugKeyboardPresentation {
              val density = LocalDensity.current
              val currentController = LocalSoftwareKeyboardPresentationController.current
              val imeBottom = WindowInsets.ime.getBottom(density).toFloat()
              SideEffect {
                controller = currentController
                imeBottomPx = imeBottom
              }
              Box(Modifier.size(width = 400.dp, height = 700.dp).testTag(ROOT_TAG)) {
                DesktopDebugKeyboard.Overlay(
                  Modifier.align(Alignment.BottomStart)
                    .padding(bottom = DESKTOP_CHROME_BOTTOM_OFFSET)
                    .testTag(KEYBOARD_TAG)
                )
              }
            }
          }
        }
      }
      waitForIdle()

      mainClock.autoAdvance = false
      val session = runOnIdle { requireNotNull(controller.acquire()) }
      runOnIdle { session.updateHiddenProgress(0.6f) }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()
      runOnIdle { session.updateHiddenProgress(0f) }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()
      waitForIdle()
      val beforeFinish = imeBottomPx

      runOnIdle { session.finish(SoftwareKeyboardPresentationEndpoint.Shown) }
      mainClock.advanceTimeByFrame()
      waitForIdle()

      assertEquals(beforeFinish, imeBottomPx, absoluteTolerance = POSITION_TOLERANCE_PX)
      assertSurfaceMatchesImeInset(imeBottomPx)
      runOnIdle {
        assertEquals(
          SoftwareKeyboardInteractionResolution.Shown,
          controller.interactionState.lastResolution,
        )
      }
    } finally {
      mainClock.autoAdvance = true
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(owner, isFocused = false)
        DesktopDebugKeyboard.registerClient(owner, null)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun hiddenOrHardwareKeyboardCannotStartAnInteractiveSession() = runComposeUiTest {
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    lateinit var controller: SoftwareKeyboardPresentationController

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
      setContent {
        ProvideSoftwareKeyboardPresentation {
          ProvideDesktopDebugKeyboardPresentation {
            val currentController = LocalSoftwareKeyboardPresentationController.current
            SideEffect { controller = currentController }
            Box(Modifier.fillMaxSize())
          }
        }
      }
      waitForIdle()

      runOnIdle { assertNull(controller.acquire()) }
      runOnIdle { DesktopDebugKeyboard.updateHardwareKeyboardConnected(true) }
      runOnIdle { assertNull(controller.acquire()) }
    } finally {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.assertSurfaceMatchesImeInset(
    imeBottomPx: Float
  ) {
    val root = onNodeWithTag(ROOT_TAG).fetchSemanticsNode().boundsInRoot
    val keyboard = onNodeWithTag(KEYBOARD_TAG).fetchSemanticsNode().boundsInRoot
    assertEquals(root.bottom - keyboard.top, imeBottomPx, absoluteTolerance = POSITION_TOLERANCE_PX)
  }
}

private class RecordingTextInputClient : TextInputClient {
  override fun requestFocus() = Unit

  override fun dismiss() = Unit
}

private val DESKTOP_CHROME_BOTTOM_OFFSET = 12.dp
private const val POSITION_TOLERANCE_PX = 1.5f
private const val ROOT_TAG = "desktop-keyboard-root"
private const val KEYBOARD_TAG = "desktop-keyboard"
