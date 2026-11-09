# 298. THE RIG'S SERVER.
#
# The rig is the app's own frontend, driven in a browser, with the Tauri
# bridge replaced by `stub.js` so nothing talks to Supabase. It is how
# 211, 290, 295 and 297 proved what they proved - and until this cycle
# it lived in a scratchpad that dies with the session, so each of those
# rebuilt it by hand first.
#
# IT SERVES `src/` AS THE DOCUMENT ROOT, which is exactly what Tauri
# does: `tauri.conf.json` says `"frontendDist": "../src"`. So the rig
# loads the REAL `index.html`, `main.js`, `map.js` and `styles.css`.
#
# **NOTHING IS COPIED.** The scratchpad version held copies of all four
# and they went stale between every cycle; that is most of why it had to
# be rebuilt rather than reused. The only file this adds to the page is
# the stub, injected as one script tag ahead of the module, and served
# from `/__rig/` so it cannot collide with anything in `src/`.
#
# WHY IT IS NOT `python -m http.server`. That reset the connection
# partway through the 515KB `main.js`, repeatedly, on two different
# ports - ERR_CONNECTION_RESET, a blank page, and no clue why. Reading
# each file into memory and sending it with a `Content-Length` fixed it
# outright. `no-store` is the other half: a cached `index.html` pointing
# at a stale module cost 297 several attempts.
#
#   npm run rig        then open http://127.0.0.1:8790
#
# Query flags, read by `stub.js` rather than here:
#   ?role=player   not the DM of the game
#   ?caster=0      the character cannot cast
#   ?break=<name>  make one loader fail - see stub.js

import os
import socketserver
import sys
from http.server import BaseHTTPRequestHandler

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
WEB = os.path.join(ROOT, "src")

STUB_URL = "/__rig/stub.js"
STUB_FILE = os.path.join(HERE, "stub.js")

# The tag the stub goes in front of. If `index.html` ever stops saying
# this, `tests/rig.smoke.mjs` fails rather than the rig silently serving
# the app with no bridge at all - which looks like a hung page.
MODULE_TAG = '<script type="module" src="/main.js" defer></script>'

TYPES = {
    ".html": "text/html; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".mjs": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".json": "application/json",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".svg": "image/svg+xml",
    ".ico": "image/x-icon",
    ".woff2": "font/woff2",
}


class Rig(BaseHTTPRequestHandler):
    # 1.1 so Content-Length is honoured and the socket is reused rather
    # than closed mid-body, which is the fault this replaced.
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def send(self, code, body=b"", ctype="text/plain; charset=utf-8"):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store, must-revalidate")
        self.end_headers()
        if body:
            try:
                self.wfile.write(body)
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                # The browser navigated away mid-send. Not a fault.
                pass

    def do_GET(self):
        path = self.path.split("?", 1)[0].split("#", 1)[0]

        if path == STUB_URL:
            with open(STUB_FILE, "rb") as f:
                return self.send(200, f.read(), TYPES[".js"])

        if path in ("", "/"):
            return self.send(200, self.index(), TYPES[".html"])

        full = os.path.normpath(os.path.join(WEB, path.lstrip("/")))
        # Nothing outside `src/` is reachable, including by `..`.
        if not full.startswith(WEB) or not os.path.isfile(full):
            return self.send(404, b"not in src/\n")

        with open(full, "rb") as f:
            body = f.read()
        ext = os.path.splitext(full)[1].lower()
        self.send(200, body, TYPES.get(ext, "application/octet-stream"))

    def index(self):
        """`src/index.html` with the stub in front of the module."""
        with open(os.path.join(WEB, "index.html"), encoding="utf-8") as f:
            html = f.read()
        if MODULE_TAG not in html:
            # LOUD RATHER THAN BLANK. Without the stub, `main.js` throws
            # on `window.__TAURI__.core` at import time and the page is
            # an empty shell with one console line - twenty minutes of
            # wondering whether the server is broken.
            return (
                "<!doctype html><meta charset=utf-8>"
                "<body style='font:16px/1.5 system-ui;padding:3rem'>"
                "<h1>The rig cannot find where to inject the stub.</h1>"
                "<p><code>src/index.html</code> no longer contains:</p>"
                "<pre>" + MODULE_TAG.replace("<", "&lt;") + "</pre>"
                "<p>Fix <code>MODULE_TAG</code> in "
                "<code>tests/rig/serve.py</code>. "
                "<code>npm test</code> checks this too.</p>"
            ).encode("utf-8")
        tag = '<script src="%s"></script>\n  ' % STUB_URL
        return html.replace(MODULE_TAG, tag + MODULE_TAG, 1).encode("utf-8")


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8790
    print("rig: serving %s at http://127.0.0.1:%d" % (WEB, port), flush=True)
    print("     ?role=player  ?caster=0  ?break=prayers", flush=True)
    Server(("127.0.0.1", port), Rig).serve_forever()
