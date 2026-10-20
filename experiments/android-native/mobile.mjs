// Owned emulator brief background/resume check; no real device.
export async function exerciseBackground({command,check,delay,evaluate,request,terminalText,shellLine}) {
  const before=await request("state");
  await command("shell","input","keyevent","KEYCODE_HOME");
  const deadline=Date.now()+10000;
  while(await evaluate("document.visibilityState!=='hidden'")){check(Date.now()<deadline,"App did not become hidden");await delay(100);}
  await delay(2000);
  const hidden=await request("state");
  check(hidden.authAttempts===before.authAttempts && hidden.channels===before.channels && hidden.inputBytes===before.inputBytes && hidden.connections===1,"Backgrounding replayed input, reauthenticated or lost the session");
  await command("shell","am","start","-n","dev.vmanyware.client/.MainActivity");
  const resumed=Date.now()+10000;
  while(await evaluate("document.visibilityState!=='visible'")){check(Date.now()<resumed,"App did not resume");await delay(100);}
  const after=await request("state");
  check(after.authAttempts===before.authAttempts && after.channels===before.channels && after.inputBytes===before.inputBytes,"Resume replayed input or reauthenticated");
  await evaluate("document.querySelector('.mobile-terminal-controls input').focus()");
  await shellLine('echo RESUME_"OK"');
  const output=Date.now()+10000;
  while(!(await terminalText()).includes("RESUME_OK")){check(Date.now()<output,"Existing SSH shell did not work after resume");await delay(100);}
  return {hiddenMs:2000,checks:["HOME hides the actual WebView", "brief background preserves one SSH session without input/auth replay", "explicit return resumes same shell with fresh output"],limitations:["Not screen lock, Doze, long suspension, OS process eviction or a background-service guarantee"]};
}

// Coordinates belong only to the reviewed Pixel 6 / Gboard English QWERTY
// fixture screenshot. Refuse other geometry; never infer keys from IME flags.
export async function exerciseKeyboard({command,check,delay,evaluate,request,terminalText}) {
  check((await command("shell","wm","size")).includes("1080x2400"),"Software-key fixture requires 1080x2400 geometry");
  const windows=await command("shell","dumpsys","window","windows");
  const imeWindow=windows.split(/\n  Window #/).find(block=>/^\d+ Window\{[^\n]* InputMethod\}:/.test(block)) || "";
  check(imeWindow.includes("package=com.google.android.inputmethod.latin"),"Software-key fixture requires Gboard");
  const region=imeWindow.match(/touchable region=SkRegion\(\(0,(\d+),1080,2400\)\)/g) || [];
  check(region.some(value=>Number(value.match(/0,(\d+),/)[1])<1900),"Full keyboard touch region is absent; IME shown flags are insufficient");
  check(await evaluate("document.activeElement?.matches('.mobile-terminal-controls input') && document.activeElement.value===''") ,"Terminal input must be focused and empty");
  const before=await request("state");
  const rows=[['qwertyuiop',54,1715],['asdfghjkl',108,1870],['zxcvbnm',216,2025]];
  async function tap(x,y){await command("shell","input","tap",String(x),String(y));await delay(120);}
  let expected='';
  for(const letter of 'echo imeox') {
    if(letter===' ')await tap(540,2180);
    else {const [row,start,y]=rows.find(([row])=>row.includes(letter));await tap(start+108*row.indexOf(letter),y);}
    expected+=letter;
    check(await evaluate(`document.querySelector('.mobile-terminal-controls input').value===${JSON.stringify(expected)}`),`Software key ${letter} did not reach the input exactly`);
  }
  await tap(1000,2025); // Gboard backspace, not the application's control button.
  expected=expected.slice(0,-1);
  check(await evaluate(`document.querySelector('.mobile-terminal-controls input').value===${JSON.stringify(expected)}`),"Gboard backspace did not remove one character");
  await tap(864,1870); // k
  await tap(1000,2180); // Gboard send action
  const deadline=Date.now()+10000;
  while(!(await terminalText()).replace(/\r/g,'').includes('\nimeok\n')){check(Date.now()<deadline,"Software-key command produced no shell output");await delay(100);}
  const after=await request("state");
  check(after.inputBytes-before.inputBytes===13,"Software keys duplicated or lost SSH input bytes");
  check(after.authAttempts===before.authAttempts && after.channels===before.channels,"Software input reconnected the session");
  check(await evaluate("document.querySelector('.mobile-terminal-controls input').value===''") ,"Gboard send did not reset local input");
  return {checks:["actual Gboard letter/space taps", "Gboard backspace", "Gboard send action", "exact 13-byte SSH input delta and real shell output"],limitations:["Reviewed owned Pixel 6 English QWERTY layout only", "Runs after rotation/control-key checks; initial key-grid visibility remains separate", "Unicode/composition and other keyboards remain unverified"]};
}
