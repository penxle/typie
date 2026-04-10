package co.typie.result

import kotlinx.coroutines.runBlocking
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class TaskDslTest {

  @Test
  fun taskEmitsPendingThenSettledOk() = runBlocking {
    val pendingValues = mutableListOf<Int>()
    var settledResult: Result<String, Nothing>? = null

    task<Int, String, Nothing> {
      emit(1)
      emit(2)
      "done"
    }.collect(
      onPending = { pendingValues.add(it) },
      onSettled = { settledResult = it },
    )

    assertEquals(listOf(1, 2), pendingValues)
    assertIs<Result.Ok<String>>(settledResult)
    assertEquals("done", (settledResult as Result.Ok).value)
  }

  @Test
  fun raiseEmitsSettledErr() = runBlocking {
    val pendingValues = mutableListOf<Unit>()
    var settledResult: Result<Nothing, String>? = null

    task<Unit, Nothing, String> {
      emit(Unit)
      raise("error")
    }.collect(
      onPending = { pendingValues.add(it) },
      onSettled = { settledResult = it },
    )

    assertEquals(1, pendingValues.size)
    assertIs<Result.Err<String>>(settledResult)
    assertEquals("error", (settledResult as Result.Err).error)
  }

  @Test
  fun exceptionEmitsSettledException() = runBlocking {
    var settledResult: Result<Nothing, Nothing>? = null

    task<Unit, Nothing, Nothing> {
      throw RuntimeException("boom")
    }.collect(
      onPending = {},
      onSettled = { settledResult = it },
    )

    assertIs<Result.Exception>(settledResult)
    assertEquals("boom", (settledResult as Result.Exception).exception.message)
  }

  @Test
  fun cancellationExceptionIsRethrown() = runBlocking {
    try {
      task<Unit, Nothing, Nothing> {
        throw kotlinx.coroutines.CancellationException("cancel")
      }.collect(
        onPending = {},
        onSettled = {},
      )
      error("should not reach")
    } catch (e: kotlinx.coroutines.CancellationException) {
      assertEquals("cancel", e.message)
    }
  }
}
