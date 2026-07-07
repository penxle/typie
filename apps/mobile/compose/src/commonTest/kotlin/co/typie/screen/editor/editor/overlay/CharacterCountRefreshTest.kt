package co.typie.screen.editor.editor.overlay

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest

class CharacterCountRefreshTest {
  @Test
  fun collapsesRapidVersionChangesIntoSingleFetch() = runTest {
    var fetchCount = 0

    val versions = flow {
      // three rapid emissions within the debounce window
      emit(1L)
      emit(2L)
      emit(3L)
    }

    val job = launch {
      collectDebouncedCharacterCounts(
        versions = versions,
        debounceMillis = 150,
        fetch = { fetchCount += 1 },
      )
    }

    job.join()

    assertEquals(1, fetchCount)
  }

  @Test
  fun separatedVersionChangesFetchEachTime() = runTest {
    var fetchCount = 0

    val versions = flow {
      emit(1L)
      kotlinx.coroutines.delay(300)
      emit(2L)
    }

    val job = launch {
      collectDebouncedCharacterCounts(
        versions = versions,
        debounceMillis = 150,
        fetch = { fetchCount += 1 },
      )
    }

    job.join()

    assertEquals(2, fetchCount)
  }
}
