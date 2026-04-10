package co.typie.result

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

class ResultTest {

  @Test
  fun okHoldsValue() {
    val result: Result<Int, String> = Result.Ok(42)
    assertIs<Result.Ok<Int>>(result)
    assertEquals(42, result.value)
  }

  @Test
  fun errHoldsError() {
    val result: Result<Int, String> = Result.Err("bad")
    assertIs<Result.Err<String>>(result)
    assertEquals("bad", result.error)
  }

  @Test
  fun exceptionHoldsThrowable() {
    val ex = RuntimeException("boom")
    val result: Result<Int, String> = Result.Exception(ex)
    assertIs<Result.Exception>(result)
    assertEquals(ex, result.exception)
  }

  @Test
  fun onOkRunsOnOkVariant() {
    var called = false
    Result.Ok(1).onOk { called = true }
    assertEquals(true, called)
  }

  @Test
  fun onOkSkipsErrVariant() {
    var called = false
    Result.Err("e").onOk<Nothing, String> { called = true }
    assertEquals(false, called)
  }

  @Test
  fun onErrRunsOnErrVariant() {
    var called = false
    Result.Err("e").onErr { called = true }
    assertEquals(true, called)
  }

  @Test
  fun onExceptionRunsOnExceptionVariant() {
    var called = false
    Result.Exception(RuntimeException()).onException { called = true }
    assertEquals(true, called)
  }

  @Test
  fun foldDispatchesCorrectly() {
    val ok: Result<Int, String> = Result.Ok(1)
    val err: Result<Int, String> = Result.Err("e")
    val ex: Result<Int, String> = Result.Exception(RuntimeException())

    assertEquals("ok:1", ok.fold({ "ok:$it" }, { "err" }, { "ex" }))
    assertEquals("err", err.fold({ "ok" }, { "err" }, { "ex" }))
    assertEquals("ex", ex.fold({ "ok" }, { "err" }, { "ex" }))
  }
}
