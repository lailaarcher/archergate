# Post to: devtalk.blender.org, Blender Artists Community, Blender Market seller forums

---

**Title:** Free licensing SDK for Blender addon developers — machine-bound keys, offline validation, open source

**Body:**

I built a software licensing SDK that lets you protect your Blender addons with machine-bound license keys. It's MIT licensed and free.

I know the Blender addon economy is under pressure. Piracy is part of it. This won't stop determined crackers, but it stops casual sharing — someone can't just copy your addon folder to another machine and have it work.

**How it works:**
- License key is bound to a SHA-256 fingerprint of the buyer's machine
- After one online validation, works offline for 30 days (no always-online requirement)
- 14-day trial built in (no server needed, no user signup)
- HMAC-signed cache + heartbeat counter to detect tampering
- Self-hosted validation server included (runs on any VPS)

**For Blender addons specifically:**
- Call the REST API from Python (your addon's `register()` function)
- Or use the C FFI if you have a compiled component
- Server is a single binary — `cargo install archergate-license-server`

**Quick Python example:**
```python
import urllib.request, json

def validate_license(key):
    data = json.dumps({
        "license_key": key,
        "machine_fingerprint": get_machine_id(),
        "plugin_id": "com.you.addon"
    }).encode()
    req = urllib.request.Request(
        "https://your-server.com/validate",
        data=data,
        headers={"Content-Type": "application/json"}
    )
    resp = json.loads(urllib.request.urlopen(req).read())
    return resp["valid"]
```

**Links:**
- GitHub: https://github.com/lailaarcher/archergate
- Sign up for beta: https://archergate.io/sdk
- crates.io (Rust): https://crates.io/crates/archergate-license

Open beta, looking for feedback. What would make this work for your addon?

---

*Note: Blender community values open source heavily. Lead with MIT license and self-hosting. Don't be salesy.*
