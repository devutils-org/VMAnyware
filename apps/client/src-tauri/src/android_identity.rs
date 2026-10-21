//! Narrow application-context JNI bridge. Initialization does not access secrets.
use jni::{
    errors::ThrowRuntimeExAndDefault,
    jni_mangle, jni_sig, jni_str,
    objects::{Global, JByteArray, JObject, JValue},
    EnvUnowned, JavaVM,
};
use std::sync::OnceLock;
use zeroize::Zeroizing;
struct Bridge {
    vm: JavaVM,
    helper: Global<JObject<'static>>,
}
static BRIDGE: OnceLock<Bridge> = OnceLock::new();
const ERROR: &str = "Android system key store unavailable or unable to open this identity. Use the passphrase or explicitly forget the saved secret before saving again.";
#[allow(non_snake_case)]
#[jni_mangle("dev.vmanyware.client.MainActivity")]
pub fn initializeIdentityVault<'caller>(
    mut env: EnvUnowned<'caller>,
    _activity: JObject<'caller>,
    helper: JObject<'caller>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        if BRIDGE.get().is_none() {
            let bridge = Bridge {
                vm: env.get_java_vm()?,
                helper: env.new_global_ref(helper)?,
            };
            let _ = BRIDGE.set(bridge);
        }
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}
pub(crate) fn save(scope: &str, secret: &[u8]) -> Result<(), String> {
    let bridge = BRIDGE.get().ok_or(ERROR)?;
    bridge
        .vm
        .attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
            let scope = env.new_string(scope)?;
            let bytes = env.byte_array_from_slice(secret)?;
            let result = env.call_method(
                &bridge.helper,
                jni_str!("save"),
                jni_sig!("(Ljava/lang/String;[B)V"),
                &[
                    JValue::Object(scope.as_ref()),
                    JValue::Object(bytes.as_ref()),
                ],
            );
            // Java clears this argument in finally, including encryption/storage failure.
            result?.v()
        })
        .map_err(|_| ERROR.into())
}
pub(crate) fn read(scope: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    let bridge = BRIDGE.get().ok_or(ERROR)?;
    bridge
        .vm
        .attach_current_thread_for_scope(|env| -> jni::errors::Result<_> {
            let scope = env.new_string(scope)?;
            let object = env
                .call_method(
                    &bridge.helper,
                    jni_str!("read"),
                    jni_sig!("(Ljava/lang/String;)[B"),
                    &[JValue::Object(scope.as_ref())],
                )?
                .l()?;
            let bytes = env.cast_local::<JByteArray>(object)?;
            let len = bytes.len(env)?;
            if len > 1024 {
                return Err(jni::errors::Error::NullPtr("Invalid unlock secret"));
            }
            let result = Zeroizing::new(env.convert_byte_array(&bytes)?);
            bytes.set_region(env, 0, &[0i8; 1024][..len])?;
            Ok(result)
        })
        .map_err(|_| ERROR.into())
}
pub(crate) fn forget(scope: &str) -> Result<(), String> {
    let bridge = BRIDGE.get().ok_or(ERROR)?;
    bridge
        .vm
        .attach_current_thread_for_scope(|env| -> jni::errors::Result<()> {
            let scope = env.new_string(scope)?;
            env.call_method(
                &bridge.helper,
                jni_str!("forget"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(scope.as_ref())],
            )?
            .v()
        })
        .map_err(|_| ERROR.into())
}
