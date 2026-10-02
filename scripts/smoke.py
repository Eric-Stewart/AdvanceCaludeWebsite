#!/usr/bin/env python3
"""
End-to-end smoke test for the chromatex front end.

Drives headless Firefox through geckodriver over plain WebDriver HTTP (no
third-party Python packages) and asserts that every control actually changes
the page: chips refetch, sliders refetch, reroll rerolls, recolour rewrites
the custom properties, the motion toggle toggles, and no LaTeX fails to
typeset along the way.

    ./target/release/chromatex --port 8099 &
    python3 scripts/smoke.py

Requires `geckodriver` and `firefox` on PATH. Exits non-zero on any failure.
"""
import base64, json, os, subprocess, sys, time, urllib.request

PORT = 4446
BASE = f"http://127.0.0.1:{PORT}"
URL = os.environ.get("CHROMATEX_URL", "http://127.0.0.1:8099/")
SHOT = os.environ.get("CHROMATEX_SHOT", "/tmp/chromatex-smoke.png")


def rq(method, path, payload=None):
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


drv = subprocess.Popen(["geckodriver", "--port", str(PORT)],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
fails = []
try:
    for _ in range(60):
        try:
            urllib.request.urlopen(BASE + "/status", timeout=2); break
        except Exception:
            time.sleep(0.5)

    sid = rq("POST", "/session", {"capabilities": {"alwaysMatch": {
        "browserName": "firefox",
        "moz:firefoxOptions": {"args": ["-headless", "-width", "1400", "-height", "1000"]},
    }}})["value"]["sessionId"]
    s = f"/session/{sid}"
    rq("POST", s + "/window/rect", {"width": 1400, "height": 1000, "x": 0, "y": 0})
    rq("POST", s + "/url", {"url": URL})
    time.sleep(4)

    def js(script, *args):
        return rq("POST", s + "/execute/sync", {"script": script, "args": list(args)})["value"]

    def check(name, cond, detail=""):
        print(("PASS  " if cond else "FAIL  ") + name + ("  " + str(detail) if detail else ""))
        if not cond:
            fails.append(name)

    # 1. Series chip switches the expansion.
    before = js("return document.getElementById('series-endpoint').textContent")
    js("document.querySelectorAll('#series-chips .chip')[8].click()")  # ln(1+x)
    time.sleep(1.5)
    after = js("return document.getElementById('series-endpoint').textContent")
    tex = js("return document.getElementById('series-display').textContent")
    check("series chip switches function", "f=log1p" in after and before != after, after)
    check("series shows ln(1+x) expansion", "ln" in tex and "x" in tex, tex[:60])

    # 2. Terms slider changes the term count.
    js("const i=document.getElementById('series-terms'); i.value=14;"
       "i.dispatchEvent(new Event('input')); i.dispatchEvent(new Event('change'));")
    time.sleep(1.5)
    check("terms slider refetches", "n=14" in
          js("return document.getElementById('series-endpoint').textContent"))

    # 3. Matrix reroll produces a different matrix.
    m1 = js("return document.getElementById('matrix-pmatrix').textContent")
    js("document.getElementById('matrix-reroll').click()")
    time.sleep(1.5)
    m2 = js("return document.getElementById('matrix-pmatrix').textContent")
    check("matrix reroll changes matrix", m1 != m2 and len(m2) > 5)

    # 4. Dimension slider changes the matrix size.
    js("const i=document.getElementById('matrix-dim'); i.value=6;"
       "i.dispatchEvent(new Event('input')); i.dispatchEvent(new Event('change'));")
    time.sleep(1.5)
    check("dimension slider applies", "n=6" in
          js("return document.getElementById('matrix-endpoint').textContent"))

    # 5. Constant chip switches the continued fraction.
    js("document.querySelectorAll('#cfrac-chips .chip')[2].click()")  # phi
    time.sleep(1.5)
    check("cfrac chip switches constant", "c=phi" in
          js("return document.getElementById('cfrac-endpoint').textContent"))

    # 6. Recolour rewrites the CSS custom properties.
    c1 = js("return document.documentElement.style.getPropertyValue('--a')")
    js("document.getElementById('shuffle-palette').click()")
    time.sleep(2.0)
    c2 = js("return document.documentElement.style.getPropertyValue('--a')")
    check("recolour changes --a", c1 != c2 and c2.startswith("#"), f"{c1} -> {c2}")
    check("all eight stops set", js(
        "return ['--void','--ink','--a','--b','--c','--d','--e','--f']"
        ".every(p=>document.documentElement.style.getPropertyValue(p).startsWith('#'))"))

    # 7. Motion toggle.
    js("document.getElementById('toggle-motion').click()")
    time.sleep(0.6)
    check("motion toggle sets no-motion",
          js("return document.body.classList.contains('no-motion')") and
          js("return document.getElementById('toggle-motion').getAttribute('aria-pressed')") == "true")
    js("document.getElementById('toggle-motion').click()")
    time.sleep(0.4)
    check("motion toggle reverts",
          not js("return document.body.classList.contains('no-motion')"))

    # 8. No LaTeX failed to typeset anywhere, after all that interaction.
    check("zero tex errors after interaction",
          js("return document.querySelectorAll('.tex-error').length") == 0)
    check("katex nodes present",
          js("return document.querySelectorAll('.katex').length") > 120,
          js("return document.querySelectorAll('.katex').length"))

    # 9. The page cannot be scrolled sideways.
    #
    # Asserting scrollWidth <= innerWidth is the obvious test and it is wrong
    # here: the drifting background fragments are randomly placed each load, so
    # the root's scrollable overflow fluctuates between loads even though
    # `body { overflow-x: hidden }` means none of it is reachable. Test the
    # property a user can actually observe instead.
    scroll = js("window.scrollTo(900, 0);"
                "const x = window.scrollX; window.scrollTo(0, 0);"
                "return [x, getComputedStyle(document.body).overflowX];")
    check("page does not scroll sideways", scroll[0] == 0 and scroll[1] == "hidden", scroll)

    png = rq("GET", s + "/screenshot")["value"]
    open(SHOT, "wb").write(base64.b64decode(png))
    print("screenshot:", SHOT)
    rq("DELETE", s)
finally:
    drv.terminate()

print("\nFAILURES:", fails if fails else "none")
sys.exit(1 if fails else 0)
