"""Static file server for dist/, with the caching headers production should use:
versioned requests (`?v=<hash>`) never change and are cached for good, everything else
is revalidated on each load.

usage: python3 serve.py PORT [ROOT]"""
import functools
import http.server
import sys

port = int(sys.argv[1])
root = sys.argv[2] if len(sys.argv) > 2 else "dist"


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        if "v=" in self.path.partition("?")[2]:
            self.send_header("Cache-Control", "public, max-age=31536000, immutable")
        else:
            self.send_header("Cache-Control", "no-cache")
        super().end_headers()


Handler.extensions_map[".wasm"] = "application/wasm"
server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", port), functools.partial(Handler, directory=root)
)
print(f"serving {root} on http://127.0.0.1:{port}/", flush=True)
server.serve_forever()
