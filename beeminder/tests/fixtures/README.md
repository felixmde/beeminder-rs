# Fixtures

This directory has two fixture sets:

- min/      Curated, stable fixtures for deterministic tests.
- recorded/ Raw recordings from real API responses (schema coverage).

Unit tests should default to min/ for stability. Use recorded/ only for
schema/regression checks or manual investigations.

Recording script output:
- beeminder/tests/recording/mitmproxy_script.py writes to recorded/.

Mock helper:
- BeeminderMock::mount_fixture(...) uses min/ by default.
- BeeminderMock::mount_fixture_in("recorded", ...) loads recorded fixtures.
