// Actual packaged IPC only. Called after the owned-emulator/reset/artifact guards.
const matches = (result, identity) => result.ok && result.value?.publicKey === identity.publicKey && result.value?.fingerprint === identity.fingerprint;
export async function prepareKeystore({invoke, check, created, phrase}) {
  check((await invoke("system_identity_support")).value === true, "Android system identity backend missing");
  check(!(await invoke("unlock_system_app_identity")).ok, "Fresh identity unexpectedly has a saved unlock secret");
  check(!(await invoke("remember_app_identity", {passphrase:phrase,confirmed:false})).ok, "Vault save skipped explicit confirmation");
  check(!(await invoke("remember_app_identity", {passphrase:`Incorrect ${phrase}`,confirmed:true})).ok, "Vault saved an incorrect identity passphrase");
  check(!(await invoke("unlock_system_app_identity")).ok, "Rejected vault save left a usable secret");
  check(matches(await invoke("remember_app_identity", {passphrase:phrase,confirmed:true}), created), "Vault save changed the application identity");
  check(matches(await invoke("unlock_system_app_identity"), created), "Vault did not unlock the saved application identity");
}
export async function finishKeystore({invoke, check, created, phrase}) {
  check(matches(await invoke("unlock_system_app_identity"), created), "Saved unlock secret or identity did not survive process restart");
  check(!(await invoke("forget_app_identity", {confirmed:false})).ok, "Vault forget skipped confirmation");
  check(matches(await invoke("unlock_system_app_identity"), created), "Unconfirmed forget removed the saved secret");
  check((await invoke("forget_app_identity", {confirmed:true})).ok, "Cannot forget the saved unlock secret");
  check(!(await invoke("unlock_system_app_identity")).ok, "Forgotten unlock secret remains usable");
  check(matches(await invoke("unlock_app_identity", {passphrase:phrase}), created), "Vault forget changed or removed the encrypted SSH identity");
  check((await invoke("forget_app_identity", {confirmed:true})).ok, "Repeated explicit forget is not idempotent");
  return {checks:["actual native Keystore dispatch", "save confirmation and correct passphrase required", "same public identity from vault before/after process restart", "forget confirmation and read refusal", "manual encrypted identity remains unchanged"], limitations:["No physical hardware/biometric assertion", "SSH system-source and activity rotation are separate gates"]};
}
