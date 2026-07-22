package co.typie.ui.component.sheet

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard
import co.typie.ext.ime
import co.typie.ext.rememberTextInputBinding
import co.typie.ext.textInputFocusable
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class SheetLayoutDesktopTest {
  @Test
  fun nonScrollingBodyExcludesImeWhileSurfaceContinuesBehindIt() = runComposeUiTest {
    val sheet = Sheet()
    val requestFocus = mutableStateOf(false)
    val wasHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var clearFocus: (() -> Unit)? = null
    var imeBottomPx = 0f

    try {
      setContent {
        SheetLayoutTestTheme {
          val density = LocalDensity.current
          val imeInsets = WindowInsets.ime
          SideEffect { imeBottomPx = imeInsets.getBottom(density).toFloat() }

          Box(Modifier.testTag(NonScrollingRootTag).size(width = 400.dp, height = 700.dp)) {
            LaunchedEffect(Unit) {
              sheet.present<Unit> {
                NonScrollingSheet(
                  requestFocus = requestFocus.value,
                  onClearFocusChanged = { clearFocus = it },
                )
              }
            }
            SheetOverlay(sheet)
          }
        }
      }

      waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(true)
        DesktopDebugKeyboard.hideKeyboardSurface()
        requestFocus.value = true
      }
      waitForIdle()
      onNodeWithTag(NonScrollingFieldTag).assertIsFocused()

      runOnIdle { DesktopDebugKeyboard.updateHardwareKeyboardConnected(false) }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val rootBounds = onNodeWithTag(NonScrollingRootTag).fetchSemanticsNode().boundsInRoot
      val layoutBounds = onNodeWithTag(NonScrollingLayoutTag).fetchSemanticsNode().boundsInRoot
      val bodyBounds = onNodeWithTag(NonScrollingBodyTag).fetchSemanticsNode().boundsInRoot
      val imeTop = rootBounds.bottom - imeBottomPx

      assertTrue(imeBottomPx > 0f)
      assertTrue(
        layoutBounds.bottom > imeTop + 0.5f,
        "Sheet surface must continue behind the software keyboard",
      )
      assertTrue(
        bodyBounds.bottom <= imeTop + 0.5f,
        "Sheet-owned IME inset must be excluded from the body viewport",
      )
    } finally {
      runOnIdle {
        clearFocus?.invoke()
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(wasHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun bottomInsetCanBeDelegatedToCaller() = runComposeUiTest {
    val sheet = Sheet()
    val includeBottomInset = mutableStateOf(false)
    val requestFocus = mutableStateOf(false)
    val wasHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var clearFocus: (() -> Unit)? = null
    var imeBottomPx = 0f

    try {
      setContent {
        SheetLayoutTestTheme {
          val density = LocalDensity.current
          val imeInsets = WindowInsets.ime
          SideEffect { imeBottomPx = imeInsets.getBottom(density).toFloat() }

          Box(Modifier.testTag(BottomInsetRootTag).size(width = 400.dp, height = 700.dp)) {
            LaunchedEffect(Unit) {
              sheet.present<Unit> {
                InsetOwnershipSheet(
                  includeBottomInset = includeBottomInset.value,
                  requestFocus = requestFocus.value,
                  onClearFocusChanged = { clearFocus = it },
                )
              }
            }
            SheetOverlay(sheet)
          }
        }
      }

      waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(true)
        DesktopDebugKeyboard.hideKeyboardSurface()
        requestFocus.value = true
      }
      waitForIdle()
      onNodeWithTag(BottomInsetFieldTag).assertIsFocused()

      val noneLayoutBounds = onNodeWithTag(BottomInsetLayoutTag).fetchSemanticsNode().boundsInRoot
      val noneBodyBounds = onNodeWithTag(BottomInsetBodyTag).fetchSemanticsNode().boundsInRoot
      assertEquals(noneLayoutBounds.bottom, noneBodyBounds.bottom, absoluteTolerance = 0.5f)

      runOnIdle { DesktopDebugKeyboard.updateHardwareKeyboardConnected(false) }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val rootBounds = onNodeWithTag(BottomInsetRootTag).fetchSemanticsNode().boundsInRoot
      val noneImeLayoutBounds =
        onNodeWithTag(BottomInsetLayoutTag).fetchSemanticsNode().boundsInRoot
      val noneImeBodyBounds = onNodeWithTag(BottomInsetBodyTag).fetchSemanticsNode().boundsInRoot
      val imeTop = rootBounds.bottom - imeBottomPx
      assertTrue(imeBottomPx > 0f)
      assertEquals(noneLayoutBounds.bottom, noneImeLayoutBounds.bottom, absoluteTolerance = 0.5f)
      assertEquals(noneBodyBounds.bottom, noneImeBodyBounds.bottom, absoluteTolerance = 0.5f)
      assertTrue(noneImeLayoutBounds.bottom > imeTop + 0.5f)

      runOnIdle { includeBottomInset.value = true }
      waitForIdle()

      val ownedInsetLayoutBounds =
        onNodeWithTag(BottomInsetLayoutTag).fetchSemanticsNode().boundsInRoot
      val ownedInsetBodyBounds = onNodeWithTag(BottomInsetBodyTag).fetchSemanticsNode().boundsInRoot
      assertEquals(
        noneImeLayoutBounds.bottom,
        ownedInsetLayoutBounds.bottom,
        absoluteTolerance = 0.5f,
      )
      assertTrue(
        ownedInsetBodyBounds.bottom <= imeTop + 0.5f,
        "Sheet-owned bottom inset must reserve the IME height",
      )
    } finally {
      runOnIdle {
        clearFocus?.invoke()
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(wasHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun focusedFieldIsRevealedWhenImeAppearsAfterFocus() = runComposeUiTest {
    val sheet = Sheet()
    val requestFocus = mutableStateOf(false)
    val wasHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var clearFocus: (() -> Unit)? = null
    var imeBottomPx = 0f

    try {
      setContent {
        SheetLayoutTestTheme {
          val density = LocalDensity.current
          val imeInsets = WindowInsets.ime
          SideEffect { imeBottomPx = imeInsets.getBottom(density).toFloat() }

          Box(Modifier.testTag(RootTag).size(width = 400.dp, height = 700.dp)) {
            LaunchedEffect(Unit) {
              sheet.present<Unit> {
                DelayedImeSheet(
                  requestFocus = requestFocus.value,
                  onClearFocusChanged = { clearFocus = it },
                )
              }
            }
            SheetOverlay(sheet)
          }
        }
      }

      waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(true)
        DesktopDebugKeyboard.hideKeyboardSurface()
        requestFocus.value = true
      }
      waitForIdle()
      onNodeWithTag(FieldTag).assertIsFocused()

      val initialFieldBounds = onNodeWithTag(FieldTag).fetchSemanticsNode().boundsInRoot
      val initialHeaderBounds = onNodeWithTag(HeaderTag).fetchSemanticsNode().boundsInRoot
      val scrollNode =
        onNode(hasScrollAction() and hasAnyAncestor(hasTestTag(LayoutTag)), useUnmergedTree = true)
      assertEquals(0f, scrollNode.verticalScrollValue(), absoluteTolerance = 0.5f)

      runOnIdle { DesktopDebugKeyboard.updateHardwareKeyboardConnected(false) }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val rootBounds = onNodeWithTag(RootTag).fetchSemanticsNode().boundsInRoot
      val fieldBounds = onNodeWithTag(FieldTag).fetchSemanticsNode().boundsInRoot
      val headerBounds = onNodeWithTag(HeaderTag).fetchSemanticsNode().boundsInRoot
      val imeTop = rootBounds.bottom - imeBottomPx
      val wholeSheetShift = headerBounds.top - initialHeaderBounds.top
      val fieldShift = fieldBounds.top - initialFieldBounds.top

      assertTrue(imeBottomPx > 0f)
      assertTrue(
        initialFieldBounds.bottom > imeTop,
        "fixture must place the initially visible field inside the final IME area",
      )
      assertTrue(fieldBounds.bottom <= imeTop + 0.5f)
      assertTrue(fieldShift < wholeSheetShift - 0.5f)
      assertTrue(scrollNode.verticalScrollValue() > 0f)
    } finally {
      runOnIdle {
        clearFocus?.invoke()
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(wasHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun manualScrollMovesOnlyBodyWithFixedHeaderAndFooter() = runComposeUiTest {
    val sheet = Sheet()

    setContent {
      SheetLayoutTestTheme {
        Box(Modifier.size(width = 400.dp, height = 700.dp)) {
          LaunchedEffect(Unit) { sheet.present<Unit> { FixedSlotsSheet() } }
          SheetOverlay(sheet)
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
    waitForIdle()

    val scrollNode =
      onNode(
        hasScrollAction() and hasAnyAncestor(hasTestTag(FixedSlotsLayoutTag)),
        useUnmergedTree = true,
      )
    val headerBounds = onNodeWithTag(FixedHeaderTag).fetchSemanticsNode().boundsInRoot
    val footerBounds = onNodeWithTag(FixedFooterTag).fetchSemanticsNode().boundsInRoot
    val bodyBounds = onNodeWithTag(BodyMarkerTag).fetchSemanticsNode().boundsInRoot

    scrollNode.performSemanticsAction(SemanticsActions.ScrollBy) { action ->
      assertTrue(action(0f, 120f))
    }
    waitUntil(timeoutMillis = 5_000) { scrollNode.verticalScrollValue() > 0f }

    assertEquals(
      headerBounds.top,
      onNodeWithTag(FixedHeaderTag).fetchSemanticsNode().boundsInRoot.top,
      absoluteTolerance = 0.5f,
    )
    assertEquals(
      footerBounds.top,
      onNodeWithTag(FixedFooterTag).fetchSemanticsNode().boundsInRoot.top,
      absoluteTolerance = 0.5f,
    )
    assertTrue(onNodeWithTag(BodyMarkerTag).fetchSemanticsNode().boundsInRoot.top < bodyBounds.top)
  }

  @Test
  fun shortSheetDoesNotScrollWhenFocusedFieldRemainsVisible() = runComposeUiTest {
    val sheet = Sheet()
    val requestFocus = mutableStateOf(false)
    val wasHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var clearFocus: (() -> Unit)? = null
    var imeBottomPx = 0f

    try {
      setContent {
        SheetLayoutTestTheme {
          val density = LocalDensity.current
          val imeInsets = WindowInsets.ime
          SideEffect { imeBottomPx = imeInsets.getBottom(density).toFloat() }

          Box(Modifier.testTag(ShortRootTag).size(width = 400.dp, height = 700.dp)) {
            LaunchedEffect(Unit) {
              sheet.present<Unit> {
                ShortSheet(
                  requestFocus = requestFocus.value,
                  onClearFocusChanged = { clearFocus = it },
                )
              }
            }
            SheetOverlay(sheet)
          }
        }
      }

      waitUntil(timeoutMillis = 5_000) { sheet.entries.isNotEmpty() }
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(true)
        DesktopDebugKeyboard.hideKeyboardSurface()
        requestFocus.value = true
      }
      waitForIdle()
      onNodeWithTag(ShortFieldTag).assertIsFocused()

      val scrollNode =
        onNode(
          hasScrollAction() and hasAnyAncestor(hasTestTag(ShortLayoutTag)),
          useUnmergedTree = true,
        )

      runOnIdle { DesktopDebugKeyboard.updateHardwareKeyboardConnected(false) }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val rootBounds = onNodeWithTag(ShortRootTag).fetchSemanticsNode().boundsInRoot
      val fieldBounds = onNodeWithTag(ShortFieldTag).fetchSemanticsNode().boundsInRoot
      val imeTop = rootBounds.bottom - imeBottomPx

      assertTrue(imeBottomPx > 0f)
      assertTrue(fieldBounds.bottom <= imeTop + 0.5f)
      assertEquals(0f, scrollNode.verticalScrollValue(), absoluteTolerance = 0.5f)
    } finally {
      runOnIdle {
        clearFocus?.invoke()
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(wasHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Composable
  context(_: SheetScope<Unit>)
  private fun NonScrollingSheet(requestFocus: Boolean, onClearFocusChanged: (() -> Unit) -> Unit) {
    SheetLayout(
      modifier = Modifier.testTag(NonScrollingLayoutTag),
      fillHeight = true,
      bodyScroll = false,
    ) {
      Box(Modifier.testTag(NonScrollingBodyTag).fillMaxSize()) {
        TestInput(
          tag = NonScrollingFieldTag,
          requestFocus = requestFocus,
          onClearFocusChanged = onClearFocusChanged,
        )
      }
    }
  }

  @Composable
  context(_: SheetScope<Unit>)
  private fun InsetOwnershipSheet(
    includeBottomInset: Boolean,
    requestFocus: Boolean,
    onClearFocusChanged: (() -> Unit) -> Unit,
  ) {
    SheetLayout(
      modifier = Modifier.testTag(BottomInsetLayoutTag),
      fillHeight = true,
      bodyScroll = false,
      includeBottomInset = includeBottomInset,
    ) {
      Box(Modifier.testTag(BottomInsetBodyTag).fillMaxSize()) {
        TestInput(
          tag = BottomInsetFieldTag,
          requestFocus = requestFocus,
          onClearFocusChanged = onClearFocusChanged,
        )
      }
    }
  }

  @Composable
  context(_: SheetScope<Unit>)
  private fun DelayedImeSheet(requestFocus: Boolean, onClearFocusChanged: (() -> Unit) -> Unit) {
    SheetLayout(
      modifier = Modifier.testTag(LayoutTag),
      header = { Box(Modifier.testTag(HeaderTag).fillMaxWidth().height(48.dp)) },
    ) {
      Spacer(Modifier.fillMaxWidth().height(240.dp))
      TestInput(
        tag = FieldTag,
        requestFocus = requestFocus,
        onClearFocusChanged = onClearFocusChanged,
      )
    }
  }

  @Composable
  context(_: SheetScope<Unit>)
  private fun FixedSlotsSheet() {
    SheetLayout(
      modifier = Modifier.testTag(FixedSlotsLayoutTag),
      header = { Box(Modifier.testTag(FixedHeaderTag).fillMaxWidth().height(48.dp)) },
      footer = { Box(Modifier.testTag(FixedFooterTag).fillMaxWidth().height(48.dp)) },
    ) {
      Box(Modifier.testTag(BodyMarkerTag).fillMaxWidth().height(48.dp))
      Spacer(Modifier.fillMaxWidth().height(800.dp))
    }
  }

  @Composable
  context(_: SheetScope<Unit>)
  private fun ShortSheet(requestFocus: Boolean, onClearFocusChanged: (() -> Unit) -> Unit) {
    SheetLayout(
      modifier = Modifier.testTag(ShortLayoutTag),
      header = { Box(Modifier.fillMaxWidth().height(48.dp)) },
    ) {
      TestInput(
        tag = ShortFieldTag,
        requestFocus = requestFocus,
        onClearFocusChanged = onClearFocusChanged,
      )
    }
  }

  @Composable
  private fun TestInput(
    tag: String,
    requestFocus: Boolean,
    onClearFocusChanged: (() -> Unit) -> Unit,
  ) {
    var value by remember { mutableStateOf("") }
    val focusManager = LocalFocusManager.current
    val binding = rememberTextInputBinding(onDismiss = { focusManager.clearFocus() })
    SideEffect { onClearFocusChanged { focusManager.clearFocus() } }
    LaunchedEffect(requestFocus) { if (requestFocus) binding.requestFocus() }

    BasicTextField(
      value = value,
      onValueChange = { value = it },
      modifier = Modifier.testTag(tag).fillMaxWidth().height(44.dp).textInputFocusable(binding),
    )
  }

  private fun androidx.compose.ui.test.SemanticsNodeInteraction.verticalScrollValue(): Float =
    fetchSemanticsNode().config[SemanticsProperties.VerticalScrollAxisRange].value()

  @Composable
  private fun SheetLayoutTestTheme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeBlurStyle provides
        HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
      content = content,
    )
  }

  private companion object {
    const val NonScrollingRootTag = "non-scrolling-sheet-root"
    const val NonScrollingLayoutTag = "non-scrolling-sheet-layout"
    const val NonScrollingBodyTag = "non-scrolling-sheet-body"
    const val NonScrollingFieldTag = "non-scrolling-sheet-field"
    const val BottomInsetRootTag = "bottom-inset-sheet-root"
    const val BottomInsetLayoutTag = "bottom-inset-sheet-layout"
    const val BottomInsetBodyTag = "bottom-inset-sheet-body"
    const val BottomInsetFieldTag = "bottom-inset-sheet-field"
    const val RootTag = "sheet-layout-root"
    const val LayoutTag = "sheet-layout"
    const val HeaderTag = "sheet-layout-header"
    const val FieldTag = "sheet-layout-field"
    const val FixedSlotsLayoutTag = "fixed-slots-sheet-layout"
    const val FixedHeaderTag = "fixed-slots-sheet-header"
    const val FixedFooterTag = "fixed-slots-sheet-footer"
    const val BodyMarkerTag = "fixed-slots-sheet-body-marker"
    const val ShortRootTag = "short-sheet-root"
    const val ShortLayoutTag = "short-sheet-layout"
    const val ShortFieldTag = "short-sheet-field"
  }
}
