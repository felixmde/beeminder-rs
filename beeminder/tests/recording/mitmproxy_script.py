#!/usr/bin/env python3
"""
mitmproxy script for recording Beeminder API responses.

Usage:
    mitmdump -s mitmproxy_script.py --set stream_large_bodies=1
"""
from mitmproxy import http
import json
import os
import re
from datetime import datetime

BEEMINDER_HOST = "www.beeminder.com"
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "fixtures", "recorded")

ENDPOINT_PATTERNS = [
    (r"^/api/v1/auth_token\.json$", "auth", "get_token"),
    (r"^/api/v1/users/[^/]+\.json$", "user", "get_user"),
    (r"^/api/v1/users/[^/]+/goals\.json$", "goals", "get_goals"),
    (r"^/api/v1/users/[^/]+/goals/archived\.json$", "goals", "get_archived"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/datapoints\.json$", "datapoints", "list"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/datapoints/create_all\.json$", "datapoints", "create_all"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/datapoints/[^/]+\.json$", "datapoints", "single"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/refresh_graph\.json$", "goals", "refresh_graph"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/shortcircuit\.json$", "danger", "shortcircuit"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/stepdown\.json$", "danger", "stepdown"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/cancel_stepdown\.json$", "danger", "cancel_stepdown"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+/uncleme\.json$", "danger", "uncleme"),
    (r"^/api/v1/users/[^/]+/goals/[^/]+\.json$", "goals", "get_goal"),
    (r"^/api/v1/charges\.json$", "danger", "charge"),
]


def classify_endpoint(path: str):
    for pattern, category, name in ENDPOINT_PATTERNS:
        if re.match(pattern, path):
            return (category, name)
    return None


def response(flow: http.HTTPFlow) -> None:
    if flow.request.host != BEEMINDER_HOST:
        return

    path = flow.request.path.split("?")[0]
    classification = classify_endpoint(path)

    if not classification:
        print(f"Unknown endpoint: {path}")
        return

    category, name = classification
    method = flow.request.method.lower()
    status = flow.response.status_code

    # Skip danger endpoints unless explicitly enabled
    if category == "danger" and os.environ.get("SKIP_DANGER"):
        print(f"Skipping danger endpoint: {path}")
        return

    # Build status suffix
    if status == 200:
        status_suffix = "valid"
    elif status == 401:
        status_suffix = "invalid_auth"
    elif status == 404:
        status_suffix = "not_found"
    else:
        status_suffix = f"error_{status}"

    fixture_name = f"{method}_{name}_{status_suffix}"

    # Parse response body
    try:
        body = json.loads(flow.response.content)
    except json.JSONDecodeError:
        body = flow.response.content.decode("utf-8", errors="replace")

    # Build fixture
    fixture = {
        "_meta": {
            "recorded_at": datetime.now().isoformat(),
            "method": flow.request.method,
            "path": path,
            "query": dict(flow.request.query),
        },
        "request": {
            "method": flow.request.method,
            "path_pattern": path_to_pattern(path),
        },
        "response": {
            "status_code": status,
            "body": body,
        },
    }

    # Save fixture
    category_dir = os.path.join(OUTPUT_DIR, category)
    os.makedirs(category_dir, exist_ok=True)

    filepath = os.path.join(category_dir, f"{fixture_name}.json")

    # Avoid overwriting - add numeric suffix if exists
    counter = 1
    base_filepath = filepath
    while os.path.exists(filepath):
        filepath = base_filepath.replace(".json", f"_{counter}.json")
        counter += 1

    with open(filepath, "w") as f:
        json.dump(fixture, f, indent=2)

    print(f"Saved: {filepath}")


def path_to_pattern(path: str) -> str:
    """Convert a path to a regex pattern for matching."""
    # Replace user IDs
    pattern = re.sub(r"/users/[^/]+", "/users/[^/]+", path)
    # Replace goal slugs
    pattern = re.sub(r"/goals/[^/]+", "/goals/[^/]+", pattern)
    # Replace datapoint IDs
    pattern = re.sub(r"/datapoints/[^/]+", "/datapoints/[^/]+", pattern)
    # Escape dots
    pattern = pattern.replace(".", "\\.")
    # Add anchors
    return f"^{pattern}$"
