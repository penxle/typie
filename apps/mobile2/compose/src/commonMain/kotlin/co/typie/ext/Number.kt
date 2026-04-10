package co.typie.ext

val Int.comma: String
  get() = toLong().comma

val Long.comma: String
  get() {
    val digits = toString()
    val startIndex = if (digits.startsWith('-')) 1 else 0
    val prefix = digits.substring(0, startIndex)
    val body = digits.substring(startIndex)

    return prefix + body.reversed().chunked(3).joinToString(",").reversed()
  }
