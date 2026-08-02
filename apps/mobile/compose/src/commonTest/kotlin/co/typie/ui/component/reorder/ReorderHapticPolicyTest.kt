package co.typie.ui.component.reorder

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ReorderHapticPolicyTest {
  @Test
  fun `first target change emits immediately and later ticks are limited to fifty milliseconds`() {
    val policy = ReorderHapticPolicy(minimumIntervalMillis = 50)

    policy.beginDrag()

    assertTrue(policy.shouldEmit(targetIndex = 1, nowMillis = 100))
    assertFalse(policy.shouldEmit(targetIndex = 2, nowMillis = 149))
    assertTrue(policy.shouldEmit(targetIndex = 3, nowMillis = 150))
  }

  @Test
  fun `unchanged target does not emit`() {
    val policy = ReorderHapticPolicy(minimumIntervalMillis = 50)

    policy.beginDrag()

    assertTrue(policy.shouldEmit(targetIndex = 1, nowMillis = 100))
    assertFalse(policy.shouldEmit(targetIndex = 1, nowMillis = 200))
  }

  @Test
  fun `suppressed target is not replayed without another target change`() {
    val policy = ReorderHapticPolicy(minimumIntervalMillis = 50)

    policy.beginDrag()

    assertTrue(policy.shouldEmit(targetIndex = 1, nowMillis = 100))
    assertFalse(policy.shouldEmit(targetIndex = 2, nowMillis = 125))
    assertFalse(policy.shouldEmit(targetIndex = 2, nowMillis = 200))
  }

  @Test
  fun `begin and end reset the policy`() {
    val policy = ReorderHapticPolicy(minimumIntervalMillis = 50)

    assertFalse(policy.shouldEmit(targetIndex = 1, nowMillis = 100))

    policy.beginDrag()
    assertTrue(policy.shouldEmit(targetIndex = 1, nowMillis = 100))

    policy.endDrag()
    assertFalse(policy.shouldEmit(targetIndex = 2, nowMillis = 125))

    policy.beginDrag()
    assertTrue(policy.shouldEmit(targetIndex = 2, nowMillis = 125))
  }
}
