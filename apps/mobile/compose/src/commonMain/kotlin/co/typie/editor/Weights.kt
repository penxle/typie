package co.typie.editor

/** CSS Fonts Level 4 §5.2 font-weight matching. `weights` must be sorted ascending. */
fun matchWeight(weights: List<Int>, target: Int): Int? {
  if (weights.isEmpty()) return null

  if (target in 400..500) {
    weights
      .firstOrNull { it in target..500 }
      ?.let {
        return it
      }
    weights
      .lastOrNull { it < target }
      ?.let {
        return it
      }
    weights
      .firstOrNull { it > 500 }
      ?.let {
        return it
      }
  } else if (target < 400) {
    weights
      .lastOrNull { it <= target }
      ?.let {
        return it
      }
    weights
      .firstOrNull { it > target }
      ?.let {
        return it
      }
  } else {
    weights
      .firstOrNull { it >= target }
      ?.let {
        return it
      }
    weights
      .lastOrNull { it < target }
      ?.let {
        return it
      }
  }

  return null
}
