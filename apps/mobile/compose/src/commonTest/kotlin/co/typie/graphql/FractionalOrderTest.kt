package co.typie.graphql

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

class FractionalOrderTest {
  @Test
  fun `next move advances the first desired mismatch using authoritative bounds`() {
    val authoritative = linkedMapOf("a" to "100", "b" to "200", "c" to "300")

    assertEquals(
      FractionalOrderMove(key = "b", lowerOrder = null, upperOrder = "100"),
      resolveNextFractionalOrderMove(authoritative, desiredKeys = listOf("b", "c", "a")),
    )
  }

  @Test
  fun `next move preserves the dragged key when it alone explains the desired order`() {
    val authoritative = linkedMapOf("a" to "100", "b" to "200", "c" to "300")

    assertEquals(
      FractionalOrderMove(key = "a", lowerOrder = "300", upperOrder = null),
      resolveNextFractionalOrderMove(
        authoritative,
        desiredKeys = listOf("b", "c", "a"),
        preferredKey = "a",
      ),
    )
  }

  @Test
  fun `next move rebases on an actual server order response`() {
    val authoritative = linkedMapOf("a" to "100", "b" to "050", "c" to "300")

    assertEquals(
      FractionalOrderMove(key = "c", lowerOrder = "050", upperOrder = "100"),
      resolveNextFractionalOrderMove(authoritative, desiredKeys = listOf("b", "c", "a")),
    )
  }

  @Test
  fun `next move is absent when authoritative order already matches`() {
    val authoritative = linkedMapOf("a" to "100", "b" to "200")

    assertNull(resolveNextFractionalOrderMove(authoritative, desiredKeys = listOf("a", "b")))
  }

  @Test
  fun `next move rejects an incomplete desired order`() {
    val authoritative = linkedMapOf("a" to "100", "b" to "200")

    assertNull(resolveNextFractionalOrderMove(authoritative, desiredKeys = listOf("a")))
  }

  @Test
  fun `both null returns MID char`() {
    assertEquals("N", midpointOrder(null, null))
  }

  @Test
  fun `upper null appends MID char to lower`() {
    assertEquals("AN", midpointOrder("A", null))
    assertEquals("ZN", midpointOrder("Z", null))
  }

  @Test
  fun `lower null and upper has non-'A' at index 0`() {
    assertEquals("G", midpointOrder(null, "M"))
  }

  @Test
  fun `lower null and upper starts with 'B'`() {
    assertEquals("AN", midpointOrder(null, "B"))
  }

  @Test
  fun `lower null and upper has leading A then B`() {
    assertEquals("AAN", midpointOrder(null, "AB"))
  }

  @Test
  fun `both non-null diff two or more`() {
    assertEquals("B", midpointOrder("A", "C"))
  }

  @Test
  fun `both non-null diff one appends MID`() {
    assertEquals("AN", midpointOrder("A", "B"))
    assertEquals("ANN", midpointOrder("AN", "B"))
  }

  @Test
  fun `lower is prefix of upper - upper second char not 'A'`() {
    assertEquals("AB", midpointOrder("A", "AC"))
  }

  @Test
  fun `lower is prefix of upper - upper second char is 'B'`() {
    assertEquals("AAN", midpointOrder("A", "AB"))
  }

  @Test
  fun `result never ends with 'A'`() {
    val results =
      listOf(
        midpointOrder(null, null),
        midpointOrder("A", null),
        midpointOrder(null, "M"),
        midpointOrder(null, "B"),
        midpointOrder("A", "C"),
        midpointOrder("A", "B"),
        midpointOrder("A", "AC"),
        midpointOrder("A", "AB"),
      )
    for (r in results) assertTrue(r.last() != 'A', "result '$r' must not end in 'A'")
  }

  @Test
  fun `upper all-A violates precondition`() {
    assertFailsWith<IllegalArgumentException> { midpointOrder(null, "A") }
    assertFailsWith<IllegalArgumentException> { midpointOrder(null, "AA") }
    // hoisted check: caller-supplied upper surfaces in error, not recursive substring
    assertFailsWith<IllegalArgumentException> { midpointOrder("A", "AA") }
  }

  @Test
  fun `lower equal or greater than upper violates precondition`() {
    assertFailsWith<IllegalArgumentException> { midpointOrder("B", "A") }
    assertFailsWith<IllegalArgumentException> { midpointOrder("A", "A") }
  }

  @Test
  fun `non-A-Z chars violate precondition`() {
    assertFailsWith<IllegalArgumentException> { midpointOrder("a", null) }
    assertFailsWith<IllegalArgumentException> { midpointOrder(null, "1") }
  }
}
