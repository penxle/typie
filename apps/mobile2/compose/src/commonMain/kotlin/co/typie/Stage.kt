package co.typie

enum class Stage {
  Local,
  Development,
  Production;

  fun abbreviation(): String =
    when (this) {
      Local -> "local"
      Development -> "dev"
      Production -> "prod"
    }

  companion object {
    val current: Stage
      get() =
        when (Konfig.STAGE) {
          "local" -> Local
          "dev" -> Development
          "prod" -> Production
          else -> error("Invalid stage: ${Konfig.STAGE}")
        }
  }
}
