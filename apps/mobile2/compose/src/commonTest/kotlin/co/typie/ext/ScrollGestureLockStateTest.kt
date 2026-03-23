package co.typie.ext

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ScrollGestureLockStateTest {

  @Test
  fun staysLockedUntilHandleIsReleased() {
    val lockState = ScrollGestureLockState()

    val handle = lockState.acquire()

    assertTrue(lockState.isLocked)

    handle.release()

    assertFalse(lockState.isLocked)
  }

  @Test
  fun nestedLocksRequireAllHandlesToRelease() {
    val lockState = ScrollGestureLockState()

    val first = lockState.acquire()
    val second = lockState.acquire()

    first.release()
    assertTrue(lockState.isLocked)

    second.release()
    assertFalse(lockState.isLocked)
  }

  @Test
  fun releaseIsIdempotent() {
    val lockState = ScrollGestureLockState()

    val handle = lockState.acquire()

    handle.release()
    handle.release()

    assertFalse(lockState.isLocked)
  }

  @Test
  fun acquireLocksImmediately() {
    val lockState = ScrollGestureLockState()

    assertFalse(lockState.isLocked)

    lockState.acquire()

    assertTrue(lockState.isLocked)
  }
}
