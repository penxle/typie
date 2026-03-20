package co.typie.graphql

import io.ktor.client.HttpClient
import org.koin.core.annotation.Single

@Single
fun httpClient(): HttpClient = HttpClient {
  followRedirects = false
}
