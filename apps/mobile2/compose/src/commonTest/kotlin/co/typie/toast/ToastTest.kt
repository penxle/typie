package co.typie.toast

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds

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
}
