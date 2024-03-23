import json
from pathlib import Path

pkg = Path("./pkg")

with open(pkg / "package.json", encoding="utf-8") as f:
    package = json.load(f)

package["type"] = "module"
package["main"] = "egm96.js"

with open(pkg / "package.json", "w", encoding="utf-8") as f:
    json.dump(package, f, indent=2)


with open(pkg / "egm96.js", encoding="utf-8") as f:
    lines = f.readlines()

patched = False
with open(pkg / "egm96.js", "w", encoding="utf-8") as f:
    for line in lines:
        if line.strip() == "input = fetch(input);":
            f.write(
                """try {
            input = await fetch(input);
        } catch (e) {
            if (!(e instanceof TypeError)) {
                throw e;
            }
            input = await (await import("node:fs/promises")).readFile(input);
        }"""
            )
            patched = True
        else:
            f.write(line)

assert patched
