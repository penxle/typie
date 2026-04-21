package co.typie.graphql

import co.typie.contract.LoadableState

sealed interface QueryState<out D> : LoadableState<D> {
  data object Loading : QueryState<Nothing>, LoadableState.Loading

  data class Success<D>(override val data: D) : QueryState<D>, LoadableState.Success<D>

  data class Error(override val exception: Throwable) : QueryState<Nothing>, LoadableState.Error
}
