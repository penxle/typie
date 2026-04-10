package co.typie.result

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class ResultDslTest {

  @Test
  fun resultReturnsOkOnSuccess() {
    val r = result<Int, String> { 42 }
    assertIs<Result.Ok<Int>>(r)
    assertEquals(42, r.value)
  }

  @Test
  fun raiseReturnsErr() {
    val r = result<Int, String> { raise("bad") }
    assertIs<Result.Err<String>>(r)
    assertEquals("bad", r.error)
  }

  @Test
  fun exceptionReturnedException() {
    val r = result<Int, String> { throw RuntimeException("boom") }
    assertIs<Result.Exception>(r)
    assertEquals("boom", r.exception.message)
  }

  @Test
  fun cancellationExceptionIsRethrown() {
    val ex = kotlinx.coroutines.CancellationException("cancel")
    try {
      result<Int, String> { throw ex }
      error("should not reach")
    } catch (e: kotlinx.coroutines.CancellationException) {
      assertEquals("cancel", e.message)
    }
  }

  @Test
  fun unwrapExtractsOkValue() {
    val inner: Result<Int, String> = Result.Ok(10)
    val r = result<Int, String> { inner.unwrap() + 5 }
    assertIs<Result.Ok<Int>>(r)
    assertEquals(15, r.value)
  }

  @Test
  fun unwrapShortCircuitsOnErr() {
    val inner: Result<Int, String> = Result.Err("fail")
    val r = result<Int, String> {
      val v = inner.unwrap()
      v + 5 // should not reach
    }
    assertIs<Result.Err<String>>(r)
    assertEquals("fail", r.error)
  }

  @Test
  fun unwrapRethrowsException() {
    val boom = RuntimeException("boom")
    val inner: Result<Int, String> = Result.Exception(boom)
    val r = result<Int, String> { inner.unwrap() }
    assertIs<Result.Exception>(r)
    assertEquals(boom, r.exception)
  }

  @Test
  fun loadingSetsAndResetsOnOk() {
    var isLoading = false
    val r = loading<Int, String>({ isLoading = it }) { 42 }
    assertIs<Result.Ok<Int>>(r)
    assertEquals(42, r.value)
    assertEquals(false, isLoading)
  }

  @Test
  fun loadingSetsAndResetsOnException() {
    var isLoading = false
    val r = loading<Int, String>({ isLoading = it }) { throw RuntimeException("boom") }
    assertIs<Result.Exception>(r)
    assertEquals(false, isLoading)
  }

  @Test
  fun loadingSetsAndResetsOnRaise() {
    var isLoading = false
    val r = loading<Int, String>({ isLoading = it }) { raise("err") }
    assertIs<Result.Err<String>>(r)
    assertEquals(false, isLoading)
  }
}
