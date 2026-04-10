package co.typie.overlay

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest

class ToastTest {

  @Test
  fun initialStateIsNull() {
    val toast = Toast()
    assertNull(toast.state.value)
  }

  @Test
  fun showSetsState() {
    val toast = Toast()
    toast.show(ToastType.Success, "저장됨")
    val state = toast.state.value!!
    assertEquals(ToastType.Success, state.type)
    assertEquals("저장됨", state.message)
  }

  @Test
  fun dismissClearsState() {
    val toast = Toast()
    toast.show(ToastType.Error, "오류")
    toast.dismiss()
    assertNull(toast.state.value)
  }

  @Test
  fun showReplacesExistingState() {
    val toast = Toast()
    toast.show(ToastType.Success, "첫번째")
    toast.show(ToastType.Error, "두번째")
    val state = toast.state.value!!
    assertEquals(ToastType.Error, state.type)
    assertEquals("두번째", state.message)
  }

  @Test
  fun adaptiveDurationShortMessage() {
    val toast = Toast()
    toast.show(ToastType.Success, "짧은 메시지")
    // "짧은 메시지" is 5 chars, under 18 → no extra duration
    assertEquals(2.seconds, toast.state.value!!.duration)
  }

  @Test
  fun adaptiveDurationLongMessage() {
    val toast = Toast()
    val longMessage = "이것은 매우 긴 메시지입니다 테스트를 위해 작성했어요"
    toast.show(ToastType.Success, longMessage)
    val extra = ((longMessage.length - 18).coerceIn(0, 100) * 12).milliseconds
    val maxExtra = 1200.milliseconds
    val expectedExtra = if (extra > maxExtra) maxExtra else extra
    assertEquals(2.seconds + expectedExtra, toast.state.value!!.duration)
  }

  @Test
  fun customDuration() {
    val toast = Toast()
    toast.show(ToastType.Notification, "알림", duration = 10.seconds)
    // "알림" is 2 chars, under 18 → no extra
    assertEquals(10.seconds, toast.state.value!!.duration)
  }

  @Test
  fun withLoadingSuccess() = runTest {
    val toast = Toast()
    val result =
      toast.withLoading("로딩 중...") {
        success("완료")
        42
      }
    assertEquals(42, result)
    val state = toast.state.value!!
    assertEquals(ToastType.Success, state.type)
    assertEquals("완료", state.message)
  }

  @Test
  fun withLoadingSuccessDefaultMessage() = runTest {
    val toast = Toast()
    toast.withLoading("작업 완료") { 42 }
    val state = toast.state.value!!
    assertEquals(ToastType.Success, state.type)
    assertEquals("작업 완료", state.message)
  }

  @Test
  fun withLoadingFailure() = runTest {
    val toast = Toast()
    assertFailsWith<CancellationException> { toast.withLoading("로딩 중...") { failure("실패했습니다") } }
    val state = toast.state.value!!
    assertEquals(ToastType.Error, state.type)
    assertEquals("실패했습니다", state.message)
  }

  @Test
  fun withLoadingUnhandledException() = runTest {
    val toast = Toast()
    assertFailsWith<IllegalStateException> {
      toast.withLoading("로딩 중...") { throw IllegalStateException("unexpected") }
    }
    val state = toast.state.value!!
    assertEquals(ToastType.Error, state.type)
    assertEquals("오류가 발생했습니다", state.message)
  }

  @Test
  fun withLoadingIdGuard() = runTest {
    val toast = Toast()
    toast.withLoading("로딩 중...") {
      toast.show(ToastType.Notification, "새 알림")
      success("완료")
      "result"
    }
    val state = toast.state.value!!
    assertEquals(ToastType.Notification, state.type)
    assertEquals("새 알림", state.message)
  }
}
