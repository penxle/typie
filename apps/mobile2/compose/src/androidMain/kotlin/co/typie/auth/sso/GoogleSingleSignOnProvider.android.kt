package co.typie.auth.sso

import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import co.typie.Konfig
import co.typie.graphql.type.SingleSignOnProvider
import co.typie.platform.ActivityContext
import com.google.android.libraries.identity.googleid.GetSignInWithGoogleOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential

actual class GoogleSingleSignOnProvider : SingleSignOnAdapter {

  context(activity: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    val option = GetSignInWithGoogleOption.Builder(Konfig.GOOGLE_SERVER_CLIENT_ID).build()
    val request = GetCredentialRequest.Builder().addCredentialOption(option).build()

    val credentialManager = CredentialManager.create(activity)
    val result = credentialManager.getCredential(activity, request)

    val googleIdTokenCredential = GoogleIdTokenCredential.createFrom(result.credential.data)

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.GOOGLE,
      params = mapOf("code" to googleIdTokenCredential.idToken),
    )
  }
}
