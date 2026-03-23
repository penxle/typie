package co.typie.graphql

sealed interface QueryState<out D> {
  data object Loading : QueryState<Nothing>
  data class Success<D>(val data: D) : QueryState<D>
  data class Error(val exception: Throwable) : QueryState<Nothing>
}
