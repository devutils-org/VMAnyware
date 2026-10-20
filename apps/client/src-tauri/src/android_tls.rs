//! Initialize Android's certificate verifier before Tauri can start networking.
use jni::{errors::ThrowRuntimeExAndDefault, jni_mangle, objects::JObject, EnvUnowned};

// Called synchronously by MainActivity before the Tauri activity starts. The
// verifier retains application-context/JVM references, not an Activity instance.
#[allow(non_snake_case)]
#[jni_mangle("dev.vmanyware.client.MainActivity")]
pub fn initializePlatformVerifier<'caller>(
    mut env: EnvUnowned<'caller>,
    _activity: JObject<'caller>,
    context: JObject<'caller>,
) {
    env.with_env(|env| rustls_platform_verifier::android::init_with_env(env, context))
        .resolve::<ThrowRuntimeExAndDefault>();
}
