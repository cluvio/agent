#!/bin/sh

if [ -z "$AGENT_KEY" ]; then
  echo "AGENT_KEY must be provided"
  exit 1
fi

if [ -z "$GATEWAY_SERVER" ]; then
  echo "AGENT_SERVER must be provided"
  exit 1
fi

printf "secret-key = \"$AGENT_KEY\"\n\n[server]\nhost = \"$GATEWAY_SERVER\"\n" > /opt/cluvio/cluvio-agent.toml
/opt/cluvio/cluvio-agent --config /opt/cluvio/cluvio-agent.toml
