# Pubky Core Python bindings

Install a published binary wheel, which includes the native library and requires
no Rust compiler:

```bash
python -m pip install --only-binary=:all: pubkycore
```

The wheel release matrix covers macOS arm64 and x86_64, Linux glibc x86_64 and
arm64, and Windows x86_64. Python 3.9 or newer is supported. If a wheel is not
available for your platform, installation fails instead of compiling Rust.

```python
import json
from pubkycore import generate_secret_key

error, payload = generate_secret_key()
if error == "true":
    raise RuntimeError(payload)
secret_key = json.loads(payload)["secret_key"]
```

Before the first PyPI publication, download the matching `.whl` from the
`Python wheels` workflow artifacts and install that file directly:

```bash
python -m pip install /path/to/pubkycore-<version>-<platform>.whl
```

For maintainers, install Maturin 1.15.0 and run `./build.sh python` from the
repository root. Wheels are written to `dist/python/`. The wheel and its generated
bindings are build outputs and are not committed. The Python package version is
read from `Cargo.toml`. A source installation (`pip install .` from the repository
root) is available for developers with Rust installed.
