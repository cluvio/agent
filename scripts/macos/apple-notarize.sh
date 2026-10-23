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

# Stapling fetches the ticket from Apple's servers, which right after notarization can fail on
# a dropped connection or a ticket not yet available (error 68, 65): try again before giving up.
for attempt in 1 2 3 4 5; do
    if xcrun stapler staple "${dmg_name}.dmg"; then
        break
    fi
    if [ "$attempt" -eq 5 ]; then
        echo "Stapling failed after $attempt attempts"
        exit 1
    fi
    echo "Stapling failed (attempt $attempt), trying again in 30 seconds"
    sleep 30
done

