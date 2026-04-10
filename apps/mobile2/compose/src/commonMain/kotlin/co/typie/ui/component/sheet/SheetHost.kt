package co.typie.ui.component.sheet

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

@Immutable
class SheetPresentation<R>(
  val spec: SheetOverlaySpec = SheetOverlaySpec(),
  val content: @Composable SheetScope<R>.() -> Unit,
)

fun <R> sheetPresentation(
  spec: SheetOverlaySpec = SheetOverlaySpec(),
  content: @Composable SheetScope<R>.() -> Unit,
): SheetPresentation<R> {
  return SheetPresentation(
    spec = spec,
    content = content,
  )
}

class SheetHostState internal constructor(
  private val presenter: SheetOverlayPresenter,
  private val launchScope: CoroutineScope,
) {
  suspend fun <R> present(
    sheet: SheetPresentation<R>,
  ): SheetResult<R> {
    return presenter.present(
      spec = sheet.spec,
      content = sheet.content,
    )
  }

  suspend fun <R> present(
    spec: SheetOverlaySpec = SheetOverlaySpec(),
    content: @Composable SheetScope<R>.() -> Unit,
  ): SheetResult<R> {
    return presenter.present(
      spec = spec,
      content = content,
    )
  }

  suspend fun <R> await(
    sheet: SheetPresentation<R>,
  ): R {
    return when (val result = present(sheet)) {
      is SheetResult.Completed -> result.value
      is SheetResult.Dismissed -> throw CancellationException("Sheet dismissed: ${result.reason}")
    }
  }

  suspend fun <R> await(
    spec: SheetOverlaySpec = SheetOverlaySpec(),
    content: @Composable SheetScope<R>.() -> Unit,
  ): R {
    return await(
      sheet = sheetPresentation(
        spec = spec,
        content = content,
      ),
    )
  }

  fun <R> show(
    sheet: SheetPresentation<R>,
    start: CoroutineStart = CoroutineStart.UNDISPATCHED,
    onResult: (SheetResult<R>) -> Unit = {},
  ): Job {
    return launchScope.launch(start = start) {
      onResult(present(sheet))
    }
  }

  fun <R> show(
    spec: SheetOverlaySpec = SheetOverlaySpec(),
    start: CoroutineStart = CoroutineStart.UNDISPATCHED,
    onResult: (SheetResult<R>) -> Unit = {},
    content: @Composable SheetScope<R>.() -> Unit,
  ): Job {
    return show(
      sheet = sheetPresentation(
        spec = spec,
        content = content,
      ),
      start = start,
      onResult = onResult,
    )
  }
}

val LocalSheetHost = staticCompositionLocalOf<SheetHostState> {
  error("No SheetHostState provided")
}
