"""Run with the installed wheel, outside the repository's binding directories."""

import json
import unittest

import pubkycore


class WheelSmokeTest(unittest.TestCase):
    def test_native_key_generation_and_roundtrip(self):
        error, payload = pubkycore.generate_secret_key()
        self.assertEqual(error, "false", payload)
        keypair = json.loads(payload)
        self.assertEqual(len(keypair["secret_key"]), 64)
        error, public_key = pubkycore.get_public_key_from_secret_key(keypair["secret_key"])
        self.assertEqual(error, "false", public_key)

    def test_typed_records_and_errors(self):
        config = pubkycore.PubkyClientConfig(
            use_testnet=False,
            testnet_host=None,
            request_timeout_ms=5000,
            read_timeout_ms=None,
            pool_max_idle_per_host=None,
            max_error_body_bytes=None,
            user_agent_extra="python-wheel-smoke",
        )
        pubkycore.configure_client(config)
        error, details = pubkycore.get_public_key_from_secret_key("invalid")
        self.assertEqual(error, "true")
        self.assertTrue(details)


if __name__ == "__main__":
    unittest.main()
