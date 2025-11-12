#!/usr/bin/env python3
"""Test Hyperliquid meta and spotMeta endpoints to understand asset indices."""

import requests
import json

API_URL = "https://api.hyperliquid.xyz/info"

def fetch_meta():
    """Fetch perpetuals metadata."""
    payload = {"type": "meta"}
    response = requests.post(API_URL, json=payload)
    return response.json()

def fetch_spot_meta():
    """Fetch spot metadata."""
    payload = {"type": "spotMeta"}
    response = requests.post(API_URL, json=payload)
    return response.json()

def main():
    print("=" * 80)
    print("FETCHING PERPETUALS META")
    print("=" * 80)

    meta = fetch_meta()
    print(json.dumps(meta, indent=2))

    # Find HYPE in perpetuals
    if "universe" in meta:
        for idx, asset in enumerate(meta["universe"]):
            if isinstance(asset, dict) and asset.get("name") == "HYPE":
                print(f"\n✅ FOUND HYPE PERP at index: {idx}")
                print(f"   Asset data: {json.dumps(asset, indent=2)}")

    print("\n" + "=" * 80)
    print("FETCHING SPOT META")
    print("=" * 80)

    spot_meta = fetch_spot_meta()
    print(json.dumps(spot_meta, indent=2))

    # Find HYPE in spot
    if "universe" in spot_meta:
        for idx, asset in enumerate(spot_meta["universe"]):
            if isinstance(asset, dict):
                tokens = asset.get("tokens", [])
                name = asset.get("name", "")
                index = asset.get("index", idx)

                # HYPE token ID is 150, USDC is 0
                if 150 in tokens or "HYPE" in name:
                    print(f"\n✅ FOUND HYPE SPOT at index: {index}")
                    print(f"   Asset data: {json.dumps(asset, indent=2)}")
                    print(f"   ORDER API ASSET VALUE: {10000 + index}")

if __name__ == "__main__":
    main()
