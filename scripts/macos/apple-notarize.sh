#!/bin/bash

set -e

executable="$1"
dmg_name="$2"
name="$(basename $executable)"

if [ -z "$executable" -o -z "$dmg_name" ]; then
    echo "usage: $0 <executable> <dmg-name>"
    exit 1
fi

if [ -z "$APPLE_DEV_ACCOUNT" ]; then
    echo 'Missing $APPLE_DEV_ACCOUNT'
    exit 1
fi

if [ -z "$APPLE_DEV_PASSWORD" ]; then
    echo 'Missing $APPLE_DEV_PASSWORD'
    exit 1
fi

xcrun notarytool submit \
    --apple-id "$APPLE_DEV_ACCOUNT" \
    --password "$APPLE_DEV_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait \
    "${dmg_name}.dmg"

xcrun stapler staple "${dmg_name}.dmg"

