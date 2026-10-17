// Isolated WebKit renderer smoke: only fixture IPC, never the native SSH bridge.
import Cocoa
import WebKit

final class ResultHandler: NSObject, WKScriptMessageHandler {
    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let result = message.body as? [String: Any] else { exit(1) }
        print(result["message"] as? String ?? "Invalid WebKit smoke result")
        exit(result["ok"] as? Bool == true ? 0 : 1)
    }
}
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let configuration = WKWebViewConfiguration()
configuration.websiteDataStore = .nonPersistent()
let result = ResultHandler()
configuration.userContentController.add(result, name: "result")
let script = try String(contentsOfFile: CommandLine.arguments[2], encoding: .utf8)
configuration.userContentController.addUserScript(WKUserScript(source: script, injectionTime: .atDocumentStart, forMainFrameOnly: true))
let frame = NSRect(x: 0, y: 0, width: 960, height: 900)
let webView = WKWebView(frame: frame, configuration: configuration)
let window = NSWindow(contentRect: frame, styleMask: [.titled], backing: .buffered, defer: false)
window.title = "VMAnyware isolated WebKit fixture"
window.contentView = webView
window.orderBack(nil)
webView.load(URLRequest(url: URL(string: CommandLine.arguments[1])!))
DispatchQueue.main.asyncAfter(deadline: .now() + 30) { print("WebKit smoke exceeded 30 seconds"); exit(1) }
app.run()
