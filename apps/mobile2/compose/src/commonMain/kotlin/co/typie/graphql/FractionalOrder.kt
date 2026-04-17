package co.typie.graphql

internal fun midpointOrder(lower: String?, upper: String?): String {
  require(lower == null || lower.all { it in 'A'..'Z' }) {
    "lower must consist solely of characters in 'A'..'Z'"
  }
  require(upper == null || upper.all { it in 'A'..'Z' }) {
    "upper must consist solely of characters in 'A'..'Z'"
  }
  require(lower == null || upper == null || lower < upper) {
    "lower ($lower) must be lexicographically less than upper ($upper)"
  }
  require(upper == null || upper.any { it != 'A' }) {
    "upper ($upper) cannot consist solely of 'A' characters"
  }

  if (lower == null && upper == null) return "N"
  if (upper == null) return lower + "N"

  if (lower == null) {
    var i = 0
    while (upper[i] == 'A') i++
    val c = upper[i]
    return if (c - 'A' >= 2) {
      upper.substring(0, i) + ('A' + (c - 'A') / 2)
    } else {
      "A".repeat(i + 1) + "N"
    }
  }

  var i = 0
  while (i < minOf(lower.length, upper.length) && lower[i] == upper[i]) i++

  if (i < lower.length && i < upper.length) {
    val diff = upper[i] - lower[i]
    return if (diff >= 2) {
      lower.substring(0, i) + (lower[i] + diff / 2)
    } else {
      lower + "N"
    }
  }

  check(i == lower.length && i < upper.length) {
    "unreachable: upper cannot be prefix of lower when lower < upper"
  }
  return lower + midpointOrder(null, upper.substring(i))
}
