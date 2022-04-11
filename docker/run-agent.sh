#!/bin/sh

# Start script for an agent inside a linux docker container.
#
# The script expects the environment variables `AGENT_SECRET_KEY` and
# `LOCATION` (or `AGENT_GATEWAY_HOST`) to be set and renders a proper
# TOML configuration file inside the container before starting the agent.

set -e

if [ -z "$AGENT_SECRET_KEY" ]; then
  echo "AGENT_SECRET_KEY must be set"
  exit 1
fi

if [ -n "$LOCATION" ]; then
  if [ "$LOCATION" = "eu" ]; then
    AGENT_GATEWAY_HOST="gateway.eu.cluvio.com"
  elif [ "$LOCATION" = "us" ]; then
    AGENT_GATEWAY_HOST="gateway.us.cluvio.com"
  else
    echo "Invalid location: $LOCATION"
    exit 1
  fi
fi

if [ -z "$AGENT_GATEWAY_HOST" ]; then
  echo "LOCATION or AGENT_GATEWAY_HOST must be set"
  exit 1
fi

cat << EOF > /opt/cluvio/cluvio-agent.toml
secret-key = "$AGENT_SECRET_KEY"

[server]
host = "$AGENT_GATEWAY_HOST"
EOF

# To ensure process signals (e.g. as sent by ctrl+c) are forwarded to the agent,
# we spawn a background process that we explicitly wait on.
/opt/cluvio/cluvio-agent --config /opt/cluvio/cluvio-agent.toml &
wait
