package co.typie.ui.state

import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class RememberPersistedStateDesktopTest {
  @Test
  fun `changing an explicit key recreates pager state at its initial page`() = runComposeUiTest {
    val owner =
      object : ViewModelStoreOwner {
        override val viewModelStore = ViewModelStore()
      }
    var pagerKey by mutableStateOf("first|second|third")
    lateinit var pagerState: PagerState

    setContent {
      CompositionLocalProvider(LocalViewModelStoreOwner provides owner) {
        pagerState = rememberPagerState(key = pagerKey, pageCount = { 3 })
      }
    }
    waitForIdle()

    runOnIdle { pagerState.requestScrollToPage(1) }
    runOnIdle { assertEquals(1, pagerState.currentPage) }

    runOnIdle { pagerKey = "second|first|third" }
    runOnIdle { assertEquals(0, pagerState.currentPage) }

    owner.viewModelStore.clear()
  }
}
