package co.typie.migration

import android.content.Context
import android.util.Base64
import java.io.File
import java.nio.charset.StandardCharsets
import java.security.KeyStore
import java.security.MessageDigest
import java.security.PrivateKey
import java.security.spec.MGF1ParameterSpec
import java.util.zip.CRC32
import javax.crypto.Cipher
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.IvParameterSpec
import javax.crypto.spec.OAEPParameterSpec
import javax.crypto.spec.PSource
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class AndroidLegacyMigrationPlatformSource(private val context: Context) :
  LegacyMigrationPlatformSource {
  override suspend fun load(): LegacyMigrationSource? =
    withContext(Dispatchers.IO) {
      val baseDirectory = findLegacyHiveDirectory()
      val authBoxBytes = baseDirectory?.readBox("auth_box")
      val preferenceBoxBytes = baseDirectory?.readBox("preference_box")
      val themeBoxBytes = baseDirectory?.readBox("theme_box")
      val base64HiveKey = readHiveEncryptionKey()

      val authBoxSource =
        if (authBoxBytes != null && base64HiveKey != null) {
          val keyCrc =
            calculateLegacyHiveKeyCrc(base64HiveKey)
              ?: return@withContext nullSource(
                preferenceBox = preferenceBoxBytes,
                themeBox = themeBoxBytes,
              )

          LegacyEncryptedHiveBoxSource(
            bytes = authBoxBytes,
            keyCrc = keyCrc,
            decryptor =
              LegacyAuthPayloadDecryptor { payload ->
                decryptLegacyHivePayload(payload, base64HiveKey)
                  ?: error("Failed to decrypt legacy Hive auth payload.")
              },
          )
        } else {
          null
        }

      LegacyMigrationSource(
          authBox = authBoxSource,
          preferenceBox = preferenceBoxBytes,
          themeBox = themeBoxBytes,
        )
        .takeIf { it.authBox != null || it.preferenceBox != null || it.themeBox != null }
    }

  private fun findLegacyHiveDirectory(): File? {
    val candidates = buildList {
      add(File(context.dataDir, FLUTTER_DATA_DIRECTORY_NAME))
      context.filesDir.parentFile?.let { add(File(it, FLUTTER_DATA_DIRECTORY_NAME)) }
      add(context.filesDir)
    }

    return candidates.firstOrNull { directory ->
      directory.exists() &&
        LEGACY_HIVE_BOX_NAMES.any { boxName -> File(directory, "$boxName.hive").exists() }
    }
  }

  private fun File.readBox(name: String): ByteArray? {
    val file = File(this, "$name.hive")
    return file.takeIf(File::exists)?.readBytes()
  }

  private fun readHiveEncryptionKey(): String? {
    val encryptedValue =
      context
        .getSharedPreferences(FLUTTER_SECURE_STORAGE_PREFS_NAME, Context.MODE_PRIVATE)
        .getString("${FLUTTER_SECURE_STORAGE_KEY_PREFIX}_$HIVE_ENCRYPTION_KEY_NAME", null)
        ?: return null

    val attempts =
      buildList {
          add(AndroidLegacySecureStorageAlgorithms.fromConfig(context))
          add(AndroidLegacySecureStorageAlgorithms.LegacyDefault)
          add(AndroidLegacySecureStorageAlgorithms.CurrentDefault)
        }
        .distinct()

    return attempts.firstNotNullOfOrNull { algorithms ->
      runCatching {
          decryptFlutterSecureStorageValue(encryptedValue = encryptedValue, algorithms = algorithms)
        }
        .getOrNull()
    }
  }

  private fun decryptFlutterSecureStorageValue(
    encryptedValue: String,
    algorithms: AndroidLegacySecureStorageAlgorithms,
  ): String {
    val wrappedKey =
      context
        .getSharedPreferences(FLUTTER_SECURE_STORAGE_KEY_PREFS_NAME, Context.MODE_PRIVATE)
        .getString(algorithms.storage.sharedPreferencesKey, null)
        ?: error("Missing wrapped key for ${algorithms.storage}.")

    val secretKey = unwrapStorageKey(wrappedKey = wrappedKey, keyAlgorithm = algorithms.key)
    val decryptedBytes =
      algorithms.storage.decrypt(
        payload = Base64.decode(encryptedValue, Base64.DEFAULT),
        secretKey = secretKey,
      )
    return decryptedBytes.toString(StandardCharsets.UTF_8)
  }

  private fun unwrapStorageKey(
    wrappedKey: String,
    keyAlgorithm: AndroidLegacyKeyCipherAlgorithm,
  ): SecretKey {
    val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE_PROVIDER).apply { load(null) }
    val privateKey =
      keyStore.getKey(keyAlgorithm.alias(context.packageName), null) as? PrivateKey
        ?: error("Missing Android keystore key for ${keyAlgorithm.alias(context.packageName)}.")

    val cipher = keyAlgorithm.createCipher()
    keyAlgorithm.parameterSpec?.let { parameterSpec ->
      cipher.init(Cipher.UNWRAP_MODE, privateKey, parameterSpec)
    } ?: cipher.init(Cipher.UNWRAP_MODE, privateKey)

    return cipher.unwrap(
      Base64.decode(wrappedKey, Base64.DEFAULT),
      AES_KEY_ALGORITHM,
      Cipher.SECRET_KEY,
    ) as SecretKey
  }

  private fun calculateLegacyHiveKeyCrc(base64HiveKey: String): Long? {
    val hiveKey =
      runCatching { Base64.decode(base64HiveKey, Base64.DEFAULT) }.getOrNull() ?: return null
    val digest = MessageDigest.getInstance("SHA-256").digest(hiveKey)
    val crc32 = CRC32().apply { update(digest) }
    return crc32.value
  }

  private fun decryptLegacyHivePayload(payload: ByteArray, base64HiveKey: String): ByteArray? {
    val hiveKey =
      runCatching { Base64.decode(base64HiveKey, Base64.DEFAULT) }.getOrNull() ?: return null
    require(payload.size >= LEGACY_HIVE_IV_SIZE) { "Encrypted Hive payload must contain an IV." }

    val cipher = Cipher.getInstance(LEGACY_HIVE_TRANSFORMATION)
    cipher.init(
      Cipher.DECRYPT_MODE,
      javax.crypto.spec.SecretKeySpec(hiveKey, AES_KEY_ALGORITHM),
      IvParameterSpec(payload.copyOfRange(0, LEGACY_HIVE_IV_SIZE)),
    )

    return cipher.doFinal(payload.copyOfRange(LEGACY_HIVE_IV_SIZE, payload.size))
  }

  private fun nullSource(preferenceBox: ByteArray?, themeBox: ByteArray?): LegacyMigrationSource? {
    return LegacyMigrationSource(authBox = null, preferenceBox = preferenceBox, themeBox = themeBox)
      .takeIf { it.preferenceBox != null || it.themeBox != null }
  }

  private data class AndroidLegacySecureStorageAlgorithms(
    val key: AndroidLegacyKeyCipherAlgorithm,
    val storage: AndroidLegacyStorageCipherAlgorithm,
  ) {
    companion object {
      val LegacyDefault =
        AndroidLegacySecureStorageAlgorithms(
          key = AndroidLegacyKeyCipherAlgorithm.RsaPkcs1,
          storage = AndroidLegacyStorageCipherAlgorithm.AesCbc,
        )
      val CurrentDefault =
        AndroidLegacySecureStorageAlgorithms(
          key = AndroidLegacyKeyCipherAlgorithm.RsaOaep,
          storage = AndroidLegacyStorageCipherAlgorithm.AesGcm,
        )

      fun fromConfig(context: Context): AndroidLegacySecureStorageAlgorithms {
        val preferences =
          context.getSharedPreferences(
            FLUTTER_SECURE_STORAGE_CONFIG_PREFS_NAME,
            Context.MODE_PRIVATE,
          )
        val key =
          when (preferences.getString(FLUTTER_SECURE_STORAGE_CONFIG_KEY_ALGORITHM, null)) {
            AndroidLegacyKeyCipherAlgorithm.RsaOaep.rawValue ->
              AndroidLegacyKeyCipherAlgorithm.RsaOaep
            else -> AndroidLegacyKeyCipherAlgorithm.RsaPkcs1
          }
        val storage =
          when (preferences.getString(FLUTTER_SECURE_STORAGE_CONFIG_STORAGE_ALGORITHM, null)) {
            AndroidLegacyStorageCipherAlgorithm.AesGcm.rawValue ->
              AndroidLegacyStorageCipherAlgorithm.AesGcm
            else -> AndroidLegacyStorageCipherAlgorithm.AesCbc
          }

        return AndroidLegacySecureStorageAlgorithms(key = key, storage = storage)
      }
    }
  }

  private enum class AndroidLegacyKeyCipherAlgorithm(val rawValue: String) {
    RsaPkcs1("RSA_ECB_PKCS1Padding"),
    RsaOaep("RSA_ECB_OAEPwithSHA_256andMGF1Padding");

    val parameterSpec
      get() =
        when (this) {
          RsaPkcs1 -> null
          RsaOaep ->
            OAEPParameterSpec("SHA-256", "MGF1", MGF1ParameterSpec.SHA1, PSource.PSpecified.DEFAULT)
        }

    fun alias(packageName: String): String {
      return when (this) {
        RsaPkcs1 -> "$packageName.FlutterSecureStoragePluginKey"
        RsaOaep -> "$packageName.FlutterSecureStoragePluginKeyOAEP"
      }
    }

    fun createCipher(): Cipher {
      val transformation =
        when (this) {
          RsaPkcs1 -> RSA_PKCS1_TRANSFORMATION
          RsaOaep -> RSA_OAEP_TRANSFORMATION
        }

      return runCatching {
          Cipher.getInstance(transformation, ANDROID_KEYSTORE_WORKAROUND_PROVIDER)
        }
        .getOrElse { Cipher.getInstance(transformation) }
    }
  }

  private enum class AndroidLegacyStorageCipherAlgorithm(
    val rawValue: String,
    val sharedPreferencesKey: String,
  ) {
    AesCbc(
      rawValue = "AES_CBC_PKCS7Padding",
      sharedPreferencesKey = "VGhpcyBpcyB0aGUga2V5IGZvciBhIHNlY3VyZSBzdG9yYWdlIEFFUyBLZXkK",
    ),
    AesGcm(
      rawValue = "AES_GCM_NoPadding",
      sharedPreferencesKey = "AESVGhpcyBpcyB0aGUga2V5IGZvciBhIHNlY3VyZSBzdG9yYWdlIEFFUyBLZXkK",
    );

    fun decrypt(payload: ByteArray, secretKey: SecretKey): ByteArray {
      val ivSize =
        when (this) {
          AesCbc -> 16
          AesGcm -> 12
        }
      require(payload.size >= ivSize) { "Encrypted secure storage payload must contain an IV." }

      val cipher =
        Cipher.getInstance(
          when (this) {
            AesCbc -> "AES/CBC/PKCS5Padding"
            AesGcm -> "AES/GCM/NoPadding"
          }
        )

      val iv = payload.copyOfRange(0, ivSize)
      val cipherText = payload.copyOfRange(ivSize, payload.size)
      val parameterSpec =
        when (this) {
          AesCbc -> IvParameterSpec(iv)
          AesGcm -> GCMParameterSpec(128, iv)
        }

      cipher.init(Cipher.DECRYPT_MODE, secretKey, parameterSpec)
      return cipher.doFinal(cipherText)
    }
  }
}

private const val AES_KEY_ALGORITHM = "AES"
private const val ANDROID_KEYSTORE_PROVIDER = "AndroidKeyStore"
private const val ANDROID_KEYSTORE_WORKAROUND_PROVIDER = "AndroidKeyStoreBCWorkaround"
private const val FLUTTER_DATA_DIRECTORY_NAME = "app_flutter"
private const val FLUTTER_SECURE_STORAGE_CONFIG_PREFS_NAME = "FlutterSecureStorageConfiguration"
private const val FLUTTER_SECURE_STORAGE_CONFIG_KEY_ALGORITHM = "FlutterSecureSAlgorithmKey"
private const val FLUTTER_SECURE_STORAGE_CONFIG_STORAGE_ALGORITHM = "FlutterSecureSAlgorithmStorage"
private const val FLUTTER_SECURE_STORAGE_KEY_PREFS_NAME = "FlutterSecureKeyStorage"
private const val FLUTTER_SECURE_STORAGE_PREFS_NAME = "FlutterSecureStorage"
private const val FLUTTER_SECURE_STORAGE_KEY_PREFIX =
  "VGhpcyBpcyB0aGUgcHJlZml4IGZvciBhIHNlY3VyZSBzdG9yYWdlCg"
private const val HIVE_ENCRYPTION_KEY_NAME = "hive_encryption_key"
private const val LEGACY_HIVE_IV_SIZE = 16
private const val LEGACY_HIVE_TRANSFORMATION = "AES/CBC/PKCS5Padding"
private val LEGACY_HIVE_BOX_NAMES = listOf("auth_box", "preference_box", "theme_box")
private const val RSA_OAEP_TRANSFORMATION = "RSA/ECB/OAEPPadding"
private const val RSA_PKCS1_TRANSFORMATION = "RSA/ECB/PKCS1Padding"
