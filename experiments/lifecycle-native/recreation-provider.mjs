// Owned adapter for the existing generated Proxmox peer; no real infrastructure.
import { readFile } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import { createProvider } from '../android-native/provider.mjs';
const configuration = JSON.parse(await readFile(process.argv[2], 'utf8'));
const check = (ok, message) => { if (!ok) throw Error(message); };
const peer = await createProvider({
  tls: { cert: await readFile(configuration.certificate), key: await readFile(configuration.key) },
  publicKey: configuration.publicKey,
  declared: Boolean(configuration.declared),
  authorization: 'PVEAPIToken=reader@pve!fixture=fixture-only-token',
  check,
  listen: server => new Promise(resolve => server.listen(0, '127.0.0.1', () => resolve(server.address().port))),
});
console.log(JSON.stringify({ port: peer.port }));
const input = createInterface({ input: process.stdin });
input.on('line', command => {
  try {
    check(command === 'report' || command === 'snapshot', 'Unknown owned provider command');
    peer.assertComplete();
    console.log(JSON.stringify({ requests: peer.requests, errors: peer.errors }));
    if (command === 'report') { input.close(); process.stdin.destroy(); peer.server.close(); peer.server.closeAllConnections(); }
  } catch (error) {
    console.log(JSON.stringify({ failure: error.message, requests: peer.requests, errors: peer.errors }));
    input.close(); process.stdin.destroy(); peer.server.close(); peer.server.closeAllConnections(); process.exitCode = 1;
  }
});
