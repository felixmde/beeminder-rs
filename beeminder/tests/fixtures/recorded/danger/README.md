# Danger Fixtures

These fixtures were recorded against real endpoints that have
side effects (charges, derails, pledge changes).

## Recording conditions:
- Test account: beeminder.com/<testuser>
- All goals set to $0 pledge before recording
- Recorded on: 2026-01-25

## Endpoints:
| Fixture | Recorded cost | Notes |
|---------|---------------|-------|
| shortcircuit_valid.json | $0 | Goal had $0 pledge |
| stepdown_valid.json | n/a | No charge, just schedules |
| stepdown_error.json | n/a | Contract not eligible for stepdown |
| cancel_stepdown_valid.json | n/a | No charge |
| cancel_stepdown_no_pending.json | n/a | No pending stepdown |

## Not recorded yet:
- uncleme_valid.json
- charge_valid.json

## Re-recording:
Only re-record if API response schema changes.
Verify pledge amounts before running!
