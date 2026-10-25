# Technical Specification — Private Communication Platform

**Status:** Architecture approved; implementation-level choices identified where necessary.

## 1. Repository, build and deployment

### 1.1 Repository

One Git repository containing a single root Nix flake and separate Rust projects.

```text
/
├── flake.nix
├── flake.lock
├── Cargo.toml
├── server/
├── client/
├── web/
├── shared/
└── nix/
```

The root flake exposes independently buildable applications, development shells and a NixOS module.

```bash
nix build .#server
nix run .#server
nix build .#client
nix run .#client
nix build .#web
nix develop .#server
```

Native server deployment uses a NixOS container with a declaratively configured systemd service.

### 1.2 Configuration

- TOML files for application settings.
- Separate server and client configurations.
- Global server settings managed through the administrator interface.
- Configuration changes validated before application.
- Persistent settings and runtime state separated.
- Development environments provided by `nix develop`.

## 2. Client applications

### 2.1 Implementation

- Rust shared core.
- egui/eframe graphical interface.
- Linux, Windows, Android and browser/WASM targets.
- Platform-specific implementations only when required.
- Web assets served by a conventional static HTTP server.

### 2.2 Compatibility

The client checks capabilities relevant to individual features.

**Linux:** Minimum supported kernel, portals, capture APIs, Vulkan availability, audio system and hardware codecs.

**Windows:** Supported Windows version, capture APIs and hardware codecs.

**Android:** Supported Android version, media capture permissions and codec capabilities.

**Browser:** Required WASM features, browser APIs, WebRTC and media capabilities.

Unsupported platforms produce an explicit compatibility error. Unsupported individual features are disabled with an explanation rather than silently failing.

Legacy compatibility is not a project objective.

## 3. Accounts and identities

### 3.1 Account creation

- Accounts can only be created or approved by the administrator.
- The administrator controls which authentication methods are available to each account.
- Possible authentication methods include passkeys/FIDO2 and Discord OAuth.
- No external identity provider is mandatory.
- Authentication credentials are independent of encryption keys.

### 3.2 Device identity

Each device generates its own post-quantum identity key pair.

- **ML-DSA-65:** Device identity signatures.
- **ML-KEM-768:** Public-key encryption/key establishment.
- **XChaCha20-Poly1305:** Symmetric content encryption.

A device signs its own encryption-key announcements. The server verifies that the device is authorized to act for its account.

Device public keys are associated with their account through server-validated registration. Other clients verify the server's authorization and the device's cryptographic signatures.

Private identity keys remain on their respective devices.

### 3.3 Key management

Every client provides settings to:

- View registered devices and key status.
- Import/export encrypted private-key backups.
- Register or replace device keys.
- Revoke a device.
- Mark a key as compromised.

The administrator can revoke any device. Users can revoke their own devices.

Compromised keys are immediately rejected for new actions, connections and key distribution.

## 4. Authentication and authorization

### 4.1 Authentication

Authentication establishes which account and device are accessing the API.

Supported authentication methods may differ by account. Device-key challenge authentication is possible without requiring persistent bearer sessions.

Authentication does not provide access to encrypted conversation content.

### 4.2 Server authority

The API server is authoritative for:

- Account membership.
- Device registration.
- Channel access.
- Message creation and deletion.
- File uploads.
- Voice participation.
- Streaming permissions.
- P2P connection authorization.
- Key registration and revocation.

Every protected action requires authorization.

### 4.3 P2P authorization

For a permitted P2P action, the server issues a short-lived, signed, action-specific credential.

Credentials identify the authorized devices, action, destination and expiration.

Peers verify these credentials before accepting connections or media. A peer without valid server authorization is rejected.

These credentials do not replace API authentication or provide message decryption keys.

## 5. Encryption and key distribution

### 5.1 Encryption requirements

- All private communication content is end-to-end encrypted.
- Server-to-client transport is encrypted using modern TLS.
- P2P communication uses authenticated encrypted transport and E2EE media protection.
- No plaintext fallback.
- No custom cryptographic primitives.

### 5.2 Conversation keys

Separate encryption contexts are maintained for:

- Direct messages.
- Group conversations.
- Channels.
- Voice sessions.
- Streaming sessions.

Authorized devices receive the necessary encryption material through authenticated, end-to-end encrypted key-distribution messages.

A device cannot register another device's encryption key without valid proof of authorization.

The API server validates key ownership, membership and revocation status before relaying key announcements.

### 5.3 Public-key caching

The server maintains a temporary RAM cache of validated public-key announcements.

- Default inactivity expiration: 24 hours.
- Configurable globally.
- Relevant authorized activity refreshes the expiration timer.
- Unrelated activity does not refresh a key.
- Revocation immediately invalidates cached entries.
- Cache loss after restart is acceptable.

The server may request a new key announcement when a required entry is unavailable.

Public-key caching does not replace delivery of encrypted conversation keys. The server does not persist private keys or plaintext conversation secrets.

### 5.4 Key rotation

Conversation encryption keys rotate every seven days by default.

Old encryption material remains available to authorized devices for decrypting unexpired messages until its retention period ends.

There is no requirement to preserve messages for offline clients beyond their normal expiration.

Revocation triggers immediate exclusion from future key distribution and new encryption epochs.

## 6. Messages and attachments

### 6.1 Message storage

The server stores encrypted message history.

- Default retention: seven days.
- Configurable global expiration.
- Expired content is deleted automatically.
- No permanent client-side message database.
- No persistent client-side attachment cache.
- No recovery of expired content.

Clients retrieve available history from the server and decrypt it locally.

A client that lacks the required key cannot decrypt a message, even when the server still retains its ciphertext.

### 6.2 Attachments

Clients normalize and compress attachments before encryption and upload.

| Content | Standardized format |
|---|---|
| Images | WebP |
| Audio | Opus in Ogg |
| Video attachments | VP9 + Opus in WebM |

Upload size limits are configurable globally. Nonconforming attachments are rejected by clients.

The server enforces encrypted payload size and authenticated metadata limits. Clients verify the actual decrypted media format and properties before playback.

### 6.3 Media processing

- Prefer hardware acceleration where available.
- Provide CPU fallback.
- Compress media before encryption.
- Enforce global video resolution and quality limits.
- Reject unsupported or out-of-specification media.
- Allow an image to contain an optional source URL, opened explicitly by the user.

Encrypted uploads are not transcoded by the server.

## 7. Voice communication

- Direct P2P connections only.
- Server-authorized participation.
- Opus audio codec.
- End-to-end encrypted media.
- ICE/STUN for connection establishment.
- No TURN relay or server-side audio forwarding.
- Failed direct connections produce an explicit error.

Voice sessions use ephemeral encryption material independent of stored conversation history.

## 8. Screen sharing and live streaming

### 8.1 Capture

- Screen and window capture.
- System audio capture where supported.
- Per-application audio isolation where supported.
- Native platform APIs preferred.
- Clear errors when a capture capability is unavailable.

### 8.2 Video

Live streaming uses codec negotiation between participants.

VP9 is preferred where practical, but compatible alternatives may be negotiated according to client capabilities.

Hardware encoding is preferred; software encoding is permitted.

### 8.3 Participant changes

When a viewer joins or leaves:

1. The server validates participation.
2. Participants exchange supported codec capabilities.
3. The stream configuration is evaluated.
4. Renegotiation occurs if necessary.
5. Encryption and authorization remain valid throughout the transition.

Unnecessary renegotiation should be avoided. A viewer may be rejected when no mutually supported configuration exists.

## 9. API and networking

### 9.1 Server

One Rust API service handles authentication, authorization, messaging, key announcements and P2P signaling.

A modern browser-compatible encrypted API transport is required. HTTPS with WebSocket support is the baseline; the Rust implementation framework remains to be selected.

### 9.2 Operation

- No separate administrative CLI or TUI.
- Administration through the application interface.
- No persistent plaintext message or media logs.
- Minimal production diagnostics.
- Additional diagnostics available in debug builds.
- Fail closed on authorization or cryptographic validation errors.

The server is the single authority for membership and permissions, not a participant in plaintext E2EE conversations.

## 10. Retention, deletion and security

### 10.1 Retention

The administrator configures expiration for server-held information, including messages, attachments, temporary authorization data and cached public keys.

Default message and attachment retention is seven days.

Authentication and account records required for ongoing operation are managed separately from expiring conversation content.

### 10.2 Revocation

Revocation must:

- Invalidate the affected device's authorization.
- Reject future actions signed by that device.
- Remove relevant cached key announcements.
- Prevent participation in new P2P sessions.
- Exclude the device from future conversation keys.

Previously obtained plaintext or encryption material cannot be remotely erased.

### 10.3 Local security

- Private keys are stored using platform-appropriate secure storage where available.
- Exported private keys are encrypted using a passphrase-derived key.
- No persistent plaintext message history on clients.
- No persistent unencrypted media cache.
- Sensitive runtime data is released when no longer needed.
- Signed requests use unique challenges or nonces to prevent replay.

### 10.4 Data protection

- Minimize stored personal information.
- Provide users access to retained account information.
- Support account deletion and applicable data requests.
- Disclose retention policies and third-party authentication dependencies.
- Avoid storing unnecessary P2P activity records.

Compliance obligations must be assessed separately from technical retention settings.

---

## Implementation decisions remaining

The following are not yet fixed and require implementation-level evaluation:

1. Rust API framework and browser-compatible transport.
2. Post-quantum cryptographic library selection and exact authenticated key-distribution protocol.
3. Group encryption protocol, including membership changes and concurrent devices.
4. Server database engine and expiration implementation.
5. Device-key storage mechanisms across operating systems and browsers.
6. Detailed media compression presets and maximum file sizes.
7. Live-stream codec negotiation policy and fallback codec.
8. Supported minimum OS, browser and hardware versions.

These are deliberately left open rather than presented as finalized technologies.
