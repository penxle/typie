package co.typie.ext

fun String.truncate(length: Int): String =
  if (length < this.length) "${this.substring(0, length)}..." else this
