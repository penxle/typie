package co.typie.graphql

data class TypieError(
  val code: String,
  override val message: String?,
) : Exception(message)

