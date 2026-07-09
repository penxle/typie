package co.typie.editor.interaction

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorPointerOwnershipTest {
  @Test
  fun `pointer lifecycle is forwarded only while owned by the boundary`() {
    val ownership = EditorPointerOwnership()

    assertFalse(ownership.owns(pointerId = 1L))
    assertFalse(ownership.hasPointers)

    ownership.acquire(pointerId = 1L)

    assertTrue(ownership.hasPointers)
    assertTrue(ownership.owns(pointerId = 1L))
    assertFalse(ownership.owns(pointerId = 2L))

    ownership.release(pointerId = 1L)

    assertFalse(ownership.hasPointers)
    assertFalse(ownership.owns(pointerId = 1L))
  }

  @Test
  fun `reset drops all owned pointers`() {
    val ownership = EditorPointerOwnership()

    ownership.acquire(pointerId = 1L)
    ownership.acquire(pointerId = 2L)
    ownership.reset()

    assertFalse(ownership.owns(pointerId = 1L))
    assertFalse(ownership.owns(pointerId = 2L))
  }
}
