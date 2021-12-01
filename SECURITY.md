# Security Overview

An agent connects and maintains a single connection to the Cluvio servers. The server is
configured with the `host` key setting in the `[server]` section of the configuration file,
e.g `gateway.us.cluvio.com`.

## TLS

The connection between Cluvio and the agent is using TLSv1.3 over TCP. As key exchange
protocol X25519 is used (ephemeral elliptic curve diffie hellman over curve 25519). The
ciphersuite is `CHACHA20_POLY1305_SHA256`, i.e. bulk encryption uses as AEAD cipher
ChaCha20 with Poly1305. Once the TLS handshake completes successfully, the Cluvio server
has authenticated itself to the agent.

## Authentication

An agent uses a single keypair for authentication purposes. The secret key is stored in
its configuration file and does not leave the system for authentication. The public key
is registered with Cluvio and is used for agent identification as well as for authentication.

The authentication protocol is based on a challenge-response approach. When an agent has
established the TLS connection to the server, the latter generates a random nonce and
encrypts it against the agent's public key. The agent is expected to send back the decrypted
value to the server to prove the possession of the corresponding private key.
If the agent fails to do so, the connection is terminated by the server. The nonce encryption
uses the "sealed boxes" encryption scheme used by libsodium, i.e the ciphertext has this
format:

```
ephemeral_pk || box(m, recipient_pk, ephemeral_sk, nonce=blake2b(ephemeral_pk || recipient_pk))
```

where

- `ephemeral_pk` is the public key part of an ephemeral keypair.
- `ephemeral_sk` is the private key part of an ephemeral keypair.
- `recipient_pk` is the public key of the agent.
- `m` is the nonce value
- `blake2b` is the BLAKE 2b cryptographic hash function.
- `box` is the [`crypto_box`][1] public key authenticated scheme from [NaCl][1] combining X25519
  as key exchange protocol with ChaCha20Poly1305 as AEAD cipher.

## Authorisation

After successful authentication of the agent, the Cluvio server checks that the agent has actually been registered with the
system. If not, the connection is terminated, otherwise the connection is fully established and
application protocol traffic is allowed.

## Upstream connections

The Cluvio Server instructs the agent to open a connection to upstream systems, bidirectionally
forwarding traffic from upstream to Cluvio. The agent can be configured to whitelist addresses
which it considers valid. For that purpose, the configuration file may contain a list of addresses in the
`allowed_addresses` key. The format of each address can be an IP network in CIDR notation in which
case any upstream IP address must lie within this network, a DNS name or a DNS pattern which is
matched according to https://datatracker.ietf.org/doc/html/rfc6265#section-5.1.3. Should the
upstream address not be whitelisted, the agent will not attempt to connect to it. By default there
are no restrictions on upstream addresses.


[1]: https://nacl.cr.yp.to/box.html

