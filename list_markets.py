#!/usr/bin/env python3
"""
List all available markets on Hyperliquid to find spot market format
"""
import requests
import json

API_URL = "https://api.hyperliquid.xyz/info"

def get_meta():
    """Get meta information including all markets"""
    payload = {"type": "meta"}
    response = requests.post(API_URL, json=payload)
    return response.json()

def get_spot_meta():
    """Get spot meta information"""
    payload = {"type": "spotMeta"}
    response = requests.post(API_URL, json=payload)
    return response.json()

def main():
    print("=" * 80)
    print("HYPERLIQUID MARKETS LIST")
    print("=" * 80)
    print()

    # Get perp markets
    print("📊 Fetching PERP markets (meta)...")
    try:
        meta = get_meta()

        if "universe" in meta:
            print(f"\n✅ Found {len(meta['universe'])} PERP markets:")
            for i, market in enumerate(meta['universe'][:10], 1):  # Show first 10
                print(f"   {i}. {market.get('name', 'N/A')}")

            # Find HYPE
            hype_markets = [m for m in meta['universe'] if 'HYPE' in m.get('name', '')]
            if hype_markets:
                print(f"\n🔍 HYPE PERP markets found:")
                for m in hype_markets:
                    print(f"   - {m.get('name')} (symbol: {m.get('name')})")
        else:
            print("   ❌ No universe data")
            print(f"   Response keys: {meta.keys()}")
    except Exception as e:
        print(f"   ❌ Error: {e}")

    print()
    print("-" * 80)
    print()

    # Get spot markets
    print("📊 Fetching SPOT markets (spotMeta)...")
    try:
        spot_meta = get_spot_meta()

        print(f"\n✅ Spot meta response:")
        print(f"   Keys: {spot_meta.keys() if isinstance(spot_meta, dict) else 'Not a dict'}")

        if isinstance(spot_meta, dict):
            if "universe" in spot_meta:
                print(f"\n✅ Found {len(spot_meta['universe'])} SPOT markets:")
                for i, market in enumerate(spot_meta['universe'][:10], 1):
                    print(f"   {i}. {market}")

                # Find HYPE spot
                hype_spots = [m for m in spot_meta['universe'] if 'HYPE' in str(m)]
                if hype_spots:
                    print(f"\n🔍 HYPE SPOT markets found:")
                    for m in hype_spots:
                        print(f"   - {m}")
                        if isinstance(m, dict):
                            print(f"     Details: {json.dumps(m, indent=6)}")
                else:
                    print(f"\n⚠️  No HYPE found in spot markets")
                    print(f"   All spot markets: {spot_meta['universe'][:5]}...")
            elif "tokens" in spot_meta:
                print(f"\n✅ Found {len(spot_meta['tokens'])} SPOT tokens:")
                for i, token in enumerate(spot_meta['tokens'][:10], 1):
                    print(f"   {i}. {token}")

                hype_tokens = [t for t in spot_meta['tokens'] if 'HYPE' in str(t)]
                if hype_tokens:
                    print(f"\n🔍 HYPE SPOT tokens found:")
                    for t in hype_tokens:
                        print(f"   - {t}")
                        if isinstance(t, dict):
                            print(f"     Details: {json.dumps(t, indent=6)}")
            else:
                print(f"\n   Full response: {json.dumps(spot_meta, indent=2)[:500]}...")
        else:
            print(f"   Response type: {type(spot_meta)}")
            print(f"   Response: {spot_meta}")

    except Exception as e:
        print(f"   ❌ Error: {e}")
        import traceback
        traceback.print_exc()

    print()
    print("=" * 80)
    print()

    # Try alternative API endpoints
    print("📊 Trying alternative: spotClearinghouseState...")
    try:
        payload = {"type": "spotClearinghouseState", "user": "0x0000000000000000000000000000000000000000"}
        response = requests.post(API_URL, json=payload)
        result = response.json()
        print(f"   Response keys: {result.keys() if isinstance(result, dict) else 'Not a dict'}")
    except Exception as e:
        print(f"   ❌ Error: {e}")

    print()
    print("=" * 80)

if __name__ == "__main__":
    main()
