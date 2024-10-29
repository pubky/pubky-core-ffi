# Pubky Mobile Python Bindings

Python bindings for the Pubky Mobile SDK.

## Installation

```bash
pip install .
```

## Usage

```python
from pubkycore import *

# Generate a new keypair
result = generate_secret_key()
if result[0] == "success":
    print(f"Generated key: {result[1]}")
else:
    print(f"Error: {result[1]}")
```
