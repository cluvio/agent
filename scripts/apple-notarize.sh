#!/bin/bash

set -e

executable="$1"
app_name="$2"

if [ -z "$executable" -o -z "$app_name" ]; then
    echo "usage: $0 <executable> <app_name>"
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

xcrun altool --notarize-app \
    --username "$APPLE_DEV_ACCOUNT" \
    --password "$APPLE_DEV_PASSWORD" \
    --file "$app_name".dmg \
    --primary-bundle-id "com.cluvio.$(basename executable)" \

uuid=$(grep RequestUUID output | awk '{print $3}')

if [ -z "$uuid" ]; then
    echo "Missing RequestUUID in notarize output."
    exit 1
fi

for i in $(seq 1 60); do
    xcrun altool --notarization-info \
        "$uuid" \
        --username "$APPLE_DEV_ACCOUNT" \
        --password "$APPLE_DEV_PASSWORD" | tee output
    case $(grep "Status:" output | awk '{print $2}') in
        "invalid")
            echo "status = invalid"
            exit 1
            ;;
        "success")
            xcrun stapler staple "$app_name".dmg
            exit 0
            ;;
        *)
            sleep 30
            ;;
    esac
done

echo "Timeout when notarizing the app."
exit 1

