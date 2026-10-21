package dev.vmanyware.client

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

// Reached only by the fixed native identity commands, never by WebView Javascript.
// Retain application context; constructing this helper does not open the vault.
class IdentityVault(context: Context) {
  private val context = context.applicationContext
  private fun preferences() = context.getSharedPreferences("ssh-unlock-v1", Context.MODE_PRIVATE)
  private fun alias(scope: String): String {
    require(scope.matches(Regex("[0-9a-f]{64}")))
    return "dev.vmanyware.client.ssh-unlock.v1.$scope"
  }
  private fun store() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
  private fun key(store: KeyStore, name: String): SecretKey =
    store.getKey(name, null) as? SecretKey ?: error("Unlock key unavailable")

  @Synchronized
  fun save(scope: String, secret: ByteArray) {
    try {
      require(secret.isNotEmpty() && secret.size <= 1024)
      val name = alias(scope)
      val preferences = preferences()
      val store = store()
      // Never silently replace an invalidated/missing key behind retained ciphertext.
      check(!preferences.contains(scope) || store.containsAlias(name))
      if (!store.containsAlias(name)) {
        KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
          init(KeyGenParameterSpec.Builder(name, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setRandomizedEncryptionRequired(true).setUserAuthenticationRequired(false).build())
          generateKey()
        }
      }
      val cipher = Cipher.getInstance("AES/GCM/NoPadding")
      cipher.init(Cipher.ENCRYPT_MODE, key(store, name))
      cipher.updateAAD(scope.toByteArray(Charsets.US_ASCII))
      check(cipher.iv.size == 12)
      val encoded = Base64.encodeToString(byteArrayOf(1) + cipher.iv + cipher.doFinal(secret), Base64.NO_WRAP)
      check(preferences.edit().putString(scope, encoded).commit())
    } finally {
      secret.fill(0)
    }
  }

  @Synchronized
  fun read(scope: String): ByteArray {
    val name = alias(scope)
    val encoded = preferences().getString(scope, null) ?: error("Saved unlock secret absent")
    require(encoded.length <= 1500)
    val bytes = Base64.decode(encoded, Base64.NO_WRAP)
    require(bytes.size in 30..1053 && bytes[0] == 1.toByte())
    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
    cipher.init(Cipher.DECRYPT_MODE, key(store(), name), GCMParameterSpec(128, bytes.copyOfRange(1, 13)))
    cipher.updateAAD(scope.toByteArray(Charsets.US_ASCII))
    val secret = cipher.doFinal(bytes, 13, bytes.size - 13)
    if (secret.isEmpty() || secret.size > 1024) { secret.fill(0); error("Invalid unlock secret") }
    return secret
  }

  @Synchronized
  fun forget(scope: String) {
    val name = alias(scope)
    val store = store()
    if (store.containsAlias(name)) store.deleteEntry(name)
    check(!store.containsAlias(name))
    check(preferences().edit().remove(scope).commit())
  }
}
