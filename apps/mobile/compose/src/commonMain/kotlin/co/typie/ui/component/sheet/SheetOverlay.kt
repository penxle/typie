package co.typie.ui.component.sheet

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.clickable
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.theme.AppTheme

@Composable
fun SheetOverlay(state: Sheet) {
  for (entry in state.entries) {
    key(entry) {
      SheetEntryOverlay(entry = entry, onResolve = { result -> state.resolveEntry(entry, result) })
    }
  }
}

@Composable
private fun SheetEntryOverlay(entry: SheetEntry<*>, onResolve: (Any?) -> Unit) {
  @Suppress("UNCHECKED_CAST") val typedEntry = entry as SheetEntry<Any?>

  val viewModelStore = remember { ViewModelStore() }
  val viewModelStoreOwner = remember {
    object : ViewModelStoreOwner {
      override val viewModelStore
        get() = viewModelStore
    }
  }
  DisposableEffect(Unit) { onDispose { viewModelStore.clear() } }

  var pendingResult by remember(entry) { mutableStateOf<Any?>(null) }
  var resolved by remember(entry) { mutableStateOf(false) }
  var dismissed by remember(entry) { mutableStateOf(false) }

  val handleDismissed: () -> Unit = {
    if (!dismissed) {
      dismissed = true
      onResolve(if (resolved) pendingResult else null)
    }
  }

  AnchoredSheetSurface(
    stops = entry.stops,
    stopPolicy = entry.stopPolicy,
    onDismissed = handleDismissed,
    scrim = { scrimAlpha ->
      Box(
        Modifier.fillMaxSize()
          .graphicsLayer { alpha = scrimAlpha }
          .background(AppTheme.colors.scrim)
          .clickable { dismiss() }
      )
    },
  ) {
    val anchoredScope = this
    val scope =
      remember(entry, anchoredScope) {
        object : SheetScope<Any?> {
          override fun complete(result: Any?) {
            pendingResult = result
            resolved = true
            anchoredScope.dismiss()
          }

          override fun dismiss() {
            anchoredScope.dismiss()
          }
        }
      }

    PlatformBackHandler(enabled = !dismissed) { anchoredScope.dismiss() }

    CompositionLocalProvider(LocalViewModelStoreOwner provides viewModelStoreOwner) {
      context(scope) { typedEntry.content() }
    }
  }
}
