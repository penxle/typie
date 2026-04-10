package co.typie.ui.component.sheet

import androidx.compose.runtime.Composable
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest

class SheetHostStateTest {

  @Test
  fun awaitReturnsCompletedValue() = runTest {
    val host = SheetHostState(
      presenter = FakeSheetOverlayPresenter(
        result = SheetResult.Completed("done"),
      ),
      launchScope = backgroundScope,
    )

    val result = host.await<String> {}

    assertEquals("done", result)
  }

  @Test
  fun awaitThrowsCancellationWhenDismissed() = runTest {
    val host = SheetHostState(
      presenter = FakeSheetOverlayPresenter(
        result = SheetResult.Dismissed(SheetDismissReason.OutsideTap),
      ),
      launchScope = backgroundScope,
    )

    assertFailsWith<CancellationException> {
      host.await<Unit> {}
    }
  }

  @Test
  fun presentForwardsSheetPresentationSpec() = runTest {
    val presenter = FakeSheetOverlayPresenter(
      result = SheetResult.Completed(Unit),
    )
    val host = SheetHostState(
      presenter = presenter,
      launchScope = backgroundScope,
    )
    val spec = SheetOverlaySpec(
      mode = SheetMode.NonModalOverlay,
      dismissPolicy = SheetDismissPolicy(outsideTap = false),
    )

    host.present(
      sheet = sheetPresentation<Unit>(
        spec = spec,
      ) {},
    )

    assertEquals(spec, presenter.lastSpec)
  }

  @Test
  fun asyncShowDeliversCompletedResultWithoutTryCatchAtCallSite() = runTest {
    val presenter = FakeSheetOverlayPresenter(
      result = SheetResult.Completed("done"),
    )
    val host = SheetHostState(
      presenter = presenter,
      launchScope = backgroundScope,
    )
    var result: SheetResult<String>? = null

    host.show(
      sheet = sheetPresentation { },
      start = CoroutineStart.UNDISPATCHED,
    ) {
      result = it
    }

    testScheduler.advanceUntilIdle()

    assertEquals(SheetResult.Completed("done"), result)
  }

  @Test
  fun asyncShowDeliversDismissedResultWithoutThrowing() = runTest {
    val presenter = FakeSheetOverlayPresenter(
      result = SheetResult.Dismissed(SheetDismissReason.OutsideTap),
    )
    val host = SheetHostState(
      presenter = presenter,
      launchScope = backgroundScope,
    )
    var result: SheetResult<Unit>? = null

    host.show(
      sheet = sheetPresentation { },
      start = CoroutineStart.UNDISPATCHED,
    ) {
      result = it
    }

    testScheduler.advanceUntilIdle()

    assertEquals(SheetResult.Dismissed(SheetDismissReason.OutsideTap), result)
  }
}

private class FakeSheetOverlayPresenter<R>(
  private val result: SheetResult<R>,
) : SheetOverlayPresenter {
  var lastSpec: SheetOverlaySpec? = null

  override suspend fun <T> present(
    spec: SheetOverlaySpec,
    content: @Composable SheetScope<T>.() -> Unit,
  ): SheetResult<T> {
    lastSpec = spec

    @Suppress("UNCHECKED_CAST")
    return result as SheetResult<T>
  }
}
