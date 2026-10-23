package dev.vmanyware.client

import android.content.Context
import android.os.Build
import android.util.Base64
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.security.KeyStore
import java.security.MessageDigest
import java.util.UUID
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

// Test APK only: no corruption/deletion hook is exposed in the application or IPC.
@RunWith(AndroidJUnit4::class)
class IdentityVaultInstrumentedTest {
  private val scopes = mutableListOf<String>()
  private val context get() = InstrumentationRegistry.getInstrumentation().targetContext
  private val preferences get() = context.getSharedPreferences("ssh-unlock-v1", Context.MODE_PRIVATE)
  private val vault get() = IdentityVault(context)
  private fun store() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
  private fun alias(scope: String) = "dev.vmanyware.client.ssh-unlock.v1.$scope"
  private fun scope(): String = MessageDigest.getInstance("SHA-256")
    .digest(("owned-vault-instrumentation-" + UUID.randomUUID()).toByteArray())
    .joinToString("") { "%02x".format(it.toInt() and 255) }.also { scopes.add(it) }
  private fun secret() = ("Disposable instrumentation phrase " + UUID.randomUUID()).toByteArray()
  private fun refused(action: () -> Unit) {
    var rejected = false
    try { action() } catch (_: Exception) { rejected = true }
    assertTrue("Operation unexpectedly accepted", rejected)
  }

  @Before
  fun requireOwnedEmulatorApproval() {
    check(InstrumentationRegistry.getArguments().getString("vmanywareOwnedVault") ==
      "vmanyware-native-smoke:emulator-5580:reset-approved") { "Explicit owned fixture approval required" }
    check(Build.HARDWARE == "ranchu" || Build.HARDWARE == "goldfish") { "Physical targets refused" }
    check(context.packageName == "dev.vmanyware.client") { "Wrong target package" }
  }

  @After
  fun removeOnlyGeneratedScopes() {
    // No enumeration or deletion of ordinary application or user credentials.
    scopes.forEach { vault.forget(it) }
  }

  @Test
  fun roundtripReopensAndForgetPreservesIsolation() {
    val scope = scope()
    val original = secret()
    val input = original.copyOf()
    vault.save(scope, input)
    assertTrue(input.all { it == 0.toByte() })
    val read = IdentityVault(context).read(scope)
    try { assertArrayEquals(original, read) } finally { read.fill(0) }
    val encoded = preferences.getString(scope, null)!!
    assertFalse(encoded.contains(original.toString(Charsets.UTF_8)))
    val unrelated = scope()
    refused { vault.read(unrelated) }
    vault.forget(scope)
    vault.forget(scope)
    assertFalse(preferences.contains(scope))
    assertFalse(store().containsAlias(alias(scope)))
    refused { vault.read(scope) }
    original.fill(0)
  }

  @Test
  fun authenticatedCiphertextRefusesTamperingAndWrongScope() {
    val first = scope()
    val second = scope()
    vault.save(first, secret())
    vault.save(second, secret())
    val encoded = preferences.getString(first, null)!!
    val bytes = Base64.decode(encoded, Base64.NO_WRAP)
    bytes[bytes.lastIndex] = (bytes.last().toInt() xor 1).toByte()
    assertTrue(preferences.edit().putString(first, Base64.encodeToString(bytes, Base64.NO_WRAP)).commit())
    refused { vault.read(first) }
    assertTrue(preferences.edit().putString(first, encoded).putString(second, encoded).commit())
    refused { vault.read(second) }
    vault.read(first).fill(0)
  }

  @Test
  fun lostWrappingKeyCannotSilentlyReplaceRetainedCiphertext() {
    val scope = scope()
    vault.save(scope, secret())
    val encoded = preferences.getString(scope, null)
    store().deleteEntry(alias(scope))
    assertFalse(store().containsAlias(alias(scope)))
    refused { vault.read(scope) }
    val input = secret()
    refused { vault.save(scope, input) }
    assertTrue(input.all { it == 0.toByte() })
    assertEquals(encoded, preferences.getString(scope, null))
    assertFalse(store().containsAlias(alias(scope)))
    vault.forget(scope)
    vault.save(scope, secret())
    vault.read(scope).fill(0)
  }

  @Test
  fun invalidScopeVersionAndBoundsFailClosed() {
    val scope = scope()
    val large = ByteArray(1025) { 65 }
    refused { vault.save(scope, large) }
    assertTrue(large.all { it == 0.toByte() })
    val input = secret()
    refused { vault.save("../outside-owned-scope", input) }
    assertTrue(input.all { it == 0.toByte() })
    assertFalse(store().containsAlias(alias(scope)))
    vault.save(scope, secret())
    val bytes = Base64.decode(preferences.getString(scope, null), Base64.NO_WRAP)
    bytes[0] = 2
    assertTrue(preferences.edit().putString(scope, Base64.encodeToString(bytes, Base64.NO_WRAP)).commit())
    refused { vault.read(scope) }
    assertTrue(preferences.edit().putString(scope, "A".repeat(1501)).commit())
    refused { vault.read(scope) }
  }
}
