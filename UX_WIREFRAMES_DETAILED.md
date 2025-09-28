# Humr Detailed Wireframes and User Journey Maps
## Revolutionary P2P Voice Communication UX Design

### User Journey Map: Complete Connection Experience

#### Journey 1: Sarah's Same-Room Connection (Target: 8 seconds)

**Context**: Sarah wants to quickly start a voice chat with her roommate Alice for cooking coordination

```
Touchpoint 1: App Launch (0-2 seconds)
User State: Motivated, wants immediate action
Interface: Clean main screen with prominent "Start Voice Chat" button
Emotions: Confident, expectant

Action: Taps "Start Voice Chat"
System Response: Button transforms with gentle animation
Transition: Immediate visual feedback, no loading states

Touchpoint 2: Lighthouse Activation (2-4 seconds)
User State: Waiting for magic to happen
Interface: Beautiful room name generation, QR code appears with animation
Emotions: Delighted by smooth transition, curious about room name

Action: Shows phone screen to Alice
System Response: QR code optimizes brightness, room name displayed clearly
Feedback: "sunset-dragon-47" easy to read and remember

Touchpoint 3: Alice Joins (4-8 seconds)
User State: Anticipating connection
Interface: "Waiting for connection..." with radar pulse animation
Emotions: Slightly anxious but hopeful

Action: Alice scans QR code with her phone
System Response: Instant discovery via Bluetooth LE proximity
Connection: Immediate P2P handshake begins

Touchpoint 4: Connected State (8 seconds)
User State: Ready to communicate
Interface: Minimal voice chat UI with clear audio levels
Emotions: Satisfied, impressed by simplicity

Total Experience: 8 seconds from intention to talking
Success Factors: Visual feedback, progressive disclosure, zero technical friction
```

#### Journey 2: Michael's Remote Family Connection (Target: 12 seconds)

**Context**: Michael wants to call his mother who lives in another country

```
Touchpoint 1: Connection Initiation (0-3 seconds)
User State: Wants reliable family communication
Interface: "Start Voice Chat" → Magic link generation
Emotions: Hopeful but slightly concerned about technical complexity

Action: Taps "Start Voice Chat", chooses "Share Link"
System Response: Generates humr://sunset-dragon-47 with sharing options
Decision: Copies link and sends via email

Touchpoint 2: Link Sharing (3-5 seconds)
User State: Explaining to less technical family member
Interface: Link copied with "Sent to Mom" confirmation
Emotions: Confident that link will work

Action: Emails link with instructions "Just click this to talk"
System Response: Email sent, lighthouse remains active
Status: "Waiting for Mom to join..."

Touchpoint 3: Mother Receives Link (5-8 seconds)
User State: Mother sees email with link
Interface: Mother's device shows Humr web interface
Emotions: Mother feels supported by simple instructions

Action: Mother clicks link on her tablet
System Response: Opens Humr directly to connection screen
Discovery: Finds Michael's lighthouse via internet connection

Touchpoint 4: Secure Connection (8-12 seconds)
User State: Both parties ready to talk
Interface: "Connected to Michael" / "Connected to Mom"
Emotions: Relief and satisfaction, family connection achieved

Total Experience: 12 seconds from Michael's action to family conversation
Success Factors: Universal link compatibility, clear instructions, reliable internet fallback
```

### Detailed Interface Wireframes

#### Wireframe Set 1: Main Application Screens

**Desktop Main Screen (Default State)**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Humr Voice Communication                                    ⚙️ Settings    │
│                                                             🔊 Audio Test   │
│                                                             ❓ Help         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                              Welcome to Humr                               │
│                         🎙️ Secure Voice Communication                      │
│                                                                             │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                                                                 │     │
│    │                    🎯 Start Voice Chat                          │     │
│    │                                                                 │     │
│    │               Begin a new secure conversation                   │     │
│    │                                                                 │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                                                                 │     │
│    │                    🔍 Join Voice Chat                           │     │
│    │                                                                 │     │
│    │              Connect to an existing conversation                │     │
│    │                                                                 │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│                                                                             │
│    Recent Connections:                                                      │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🟢 Alice Thompson                              2 minutes ago    │     │
│    │     sunset-dragon-47 • Same room • 15ms latency                 │ ⟲   │
│    └─────────────────────────────────────────────────────────────────┘     │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🟡 Bob Chen                                   1 hour ago        │     │
│    │     ocean-phoenix-23 • Same network • 28ms latency              │ ⟲   │
│    └─────────────────────────────────────────────────────────────────┘     │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  ⚪ Carol Martinez                             Yesterday          │     │
│    │     forest-tiger-91 • Internet • 67ms latency                   │ ⟲   │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🔐 End-to-End Encrypted • 🌐 Peer-to-Peer • 🛡️ Privacy First • No Servers │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Mobile Main Screen (iOS/Android Responsive)**
```
┌─────────────────────────────┐
│  🎙️ Humr         🔊 ⚙️ ❓   │
├─────────────────────────────┤
│                             │
│      Secure Voice Chat      │
│                             │
│  ┌─────────────────────────┐ │
│  │                         │ │
│  │    🎯 Start Voice       │ │
│  │        Chat             │ │
│  │                         │ │
│  │   Begin new conversation│ │
│  │                         │ │
│  └─────────────────────────┘ │
│                             │
│  ┌─────────────────────────┐ │
│  │                         │ │
│  │    🔍 Join Voice        │ │
│  │        Chat             │ │
│  │                         │ │
│  │   Connect to existing   │ │
│  │                         │ │
│  └─────────────────────────┘ │
│                             │
│  Recent:                    │
│  ┌─────────────────────────┐ │
│  │ 🟢 Alice • 2m           │ │
│  │    sunset-dragon-47     │ │
│  └─────────────────────────┘ │
│  ┌─────────────────────────┐ │
│  │ 🟡 Bob • 1h             │ │
│  │    ocean-phoenix-23     │ │
│  └─────────────────────────┘ │
│                             │
├─────────────────────────────┤
│ 🔐 Encrypted • 🌐 P2P       │
└─────────────────────────────┘
```

#### Wireframe Set 2: Connection Setup Flow

**Lighthouse Mode - Desktop (After "Start Voice Chat")**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back to Main                Humr                            × Close Chat  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          🌟 Ready for Connection                            │
│                                                                             │
│    Your voice chat is ready! Share this with anyone to start talking:      │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                        sunset-dragon-47                         │     │
│    │                                                                 │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │   ████████████████    ████████████████    ████████████████     │     │
│    │                         [QR Code]                              │     │
│    │                                                                 │     │
│    │                    📱 Tap to enlarge for scanning               │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    Or share this magic link:                                               │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  humr://sunset-dragon-47                                        │ 📋  │
│    │  "Click this link to join my voice chat!"                       │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    Share via:                                                              │
│    💬 Messages   📧 Email   📱 WhatsApp   📲 Copy Link   🐦 Twitter        │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │   🔍 Scanning for connections...                                │     │
│    │   ◉ ◯ ◯ ◯ ◯ ◯ ◯ ◯  (animated radar sweep)                      │     │
│    │   • Bluetooth: Ready for same-room connections                  │     │
│    │   • WiFi: Broadcasting on "Home_Network_5G"                     │     │
│    │   • Internet: UPnP configured, port 41287 open                  │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Status: 🟢 Broadcasting • 🔐 End-to-End Encrypted • ⏱️ 03:47 active        │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Discovery Mode - Desktop (After "Join Voice Chat")**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back to Main                Humr                            × Close       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                          🔍 Finding Voice Chats                             │
│                                                                             │
│    Connect by scanning a QR code:                                          │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                                                                 │     │
│    │               📸 Scan QR Code                                   │     │
│    │                                                                 │     │
│    │           [Camera viewfinder area]                              │     │
│    │                                                                 │     │
│    │        Point your camera at a Humr QR code                     │     │
│    │                                                                 │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    Or connect to nearby chats:                                             │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🟢 sunset-dragon-47                                ◉ ◯ ◯ 15ms  │ →  │
│    │     Alice Thompson's Voice Chat                                 │     │
│    │     Same room • Bluetooth LE • Excellent quality               │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🟡 ocean-phoenix-23                               ◉ ◉ ◯ 28ms   │ →  │
│    │     Bob Chen's Voice Chat                                       │     │
│    │     Same network • WiFi • Good quality                          │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  ⚪ forest-tiger-91                               ◉ ◉ ◉ 67ms    │ →  │
│    │     Carol Martinez's Voice Chat                                 │     │
│    │     Internet • Encrypted • Acceptable quality                   │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    Or enter a magic link manually:                                         │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🔗 humr://room-name-here                                       │ →  │
│    │     Paste or type a Humr magic link                             │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Searching: 🔵 Bluetooth • 📶 WiFi • 🌐 Internet • 🔐 Secure connections    │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Wireframe Set 3: Active Voice Chat Interface

**Voice Chat Session - Minimal UI Focus**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  sunset-dragon-47          🔊 Volume: 75%               📞 End Chat         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                                                                             │
│                       🟢 Connected to Alice Thompson                        │
│                              Same room • 15ms latency                       │
│                                                                             │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                     🎙️ YOU ARE TALKING                          │     │
│    │   ████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░       │     │
│    │                      Your voice level                           │     │
│    │                                                                 │     │
│    │   🟢 Noise suppression active • Echo cancellation on           │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                    🔊 ALICE IS TALKING                          │     │
│    │   ░░░░░░░░░░████████████████████████████████████░░░░░░░░░░       │     │
│    │                    Alice's voice level                          │     │
│    │                                                                 │     │
│    │   🔐 End-to-end encrypted • 🛡️ Forward secrecy active          │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│                                                                             │
│                                                                             │
│         ┌─────┐    ┌─────┐    ┌─────┐    ┌─────┐    ┌─────┐               │
│         │ 🎙️ │    │ 🔇  │    │ ⚙️  │    │ 💬  │    │ 📞  │               │
│         │Mute │    │Deaf │    │Set  │    │Chat │    │End  │               │
│         │     │    │     │    │     │    │     │    │     │               │
│         └─────┘    └─────┘    └─────┘    └─────┘    └─────┘               │
│                                                                             │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🔐 Secure • 🟢 15ms latency • 📶 Excellent • 64kbps • 🔋 2h remaining      │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Voice Chat Session - Mobile Responsive**
```
┌─────────────────────────────┐
│ sunset-dragon-47      📞 End│
├─────────────────────────────┤
│                             │
│   🟢 Connected to Alice     │
│      Same room • 15ms       │
│                             │
│  ┌─────────────────────────┐ │
│  │    🎙️ YOU TALKING       │ │
│  │ ████████████░░░░░░░░░   │ │
│  │     Your voice          │ │
│  └─────────────────────────┘ │
│                             │
│  ┌─────────────────────────┐ │
│  │   🔊 ALICE TALKING      │ │
│  │ ░░░░████████████████░   │ │
│  │    Alice's voice        │ │
│  └─────────────────────────┘ │
│                             │
│                             │
│      🎙️   🔇   ⚙️   💬      │
│     Mute  Deaf  Set  Chat   │
│                             │
│                             │
├─────────────────────────────┤
│ 🔐 Secure • 🟢 15ms • 64k   │
└─────────────────────────────┘
```

### Interaction State Diagrams

#### Connection Flow State Machine

```
[App Launch]
     ↓
[Main Screen: Ready]
     ↓
[User Choice: Start or Join]
     ↓ Start                    ↓ Join
[Lighthouse Setup]         [Discovery Mode]
     ↓                          ↓
[Broadcasting State]       [Scanning State]
     ↓                          ↓
[Waiting for Connection] ← [Connection Found]
     ↓                          ↓
[Handshake in Progress] → [Joining Connection]
     ↓                          ↓
[Security Establishment]
     ↓
[Voice Chat Active]
     ↓
[Connection Ended] → [Back to Main Screen]
```

#### Discovery Priority State Flow

```
[Discovery Started]
     ↓
[Bluetooth LE Scan] → [Found Device] → [Connect via BLE]
     ↓ No devices (2s timeout)
[mDNS Network Scan] → [Found Service] → [Connect via WiFi]
     ↓ No services (3s timeout)
[Internet Magic Link] → [Valid Link] → [Connect via Internet]
     ↓ Invalid/timeout
[Manual Configuration] → [User Input] → [Connect via Manual]
     ↓ Failed
[Technical Support Mode]
```

#### Audio Quality Adaptation State Flow

```
[Connection Established]
     ↓
[Quality Assessment] → [High Quality] → [64-128kbps Mode]
     ↓ Medium                           ↓ Low
[32-64kbps Mode]                   [16-32kbps Mode]
     ↓                                  ↓
[Monitor Quality] ← ← ← ← ← ← ← ← ← ← ← ← ←
     ↓ Quality Change
[Adapt Bitrate] → [Update UI Indicators]
     ↓
[Continue Monitoring]
```

### Mobile-Specific Interaction Patterns

#### iOS Integration Points

**Share Sheet Integration**:
```
Magic Link Generated → iOS Share Sheet → Multiple Sharing Options
                    ↓                  ↓
              "humr://room-47"    Messages, Mail, WhatsApp,
                                  AirDrop, Twitter, etc.
```

**Notification Handling**:
```
Incoming Connection → iOS Notification → Tap to Accept
Background Mode → CallKit Integration → System-level Call UI
Audio Interruption → Pause, Resume → Seamless Transition
```

#### Android Integration Points

**Share Intent Integration**:
```
Magic Link Generated → Android Share Intent → App Chooser
                    ↓                       ↓
              "humr://room-47"         Messages, Gmail, WhatsApp,
                                      Bluetooth, NFC, etc.
```

**Background Audio Management**:
```
Audio Focus Request → System Audio Manager → Background Permission
Voice Chat Active → Lock Screen Controls → Media Session API
```

### Progressive Web App Considerations

#### Service Worker Integration
```
First Load → Cache Core App → Enable Offline Mode
Network Change → Update Connection Status → Retry Logic
Push Notification → Wake Sleeping Tab → Resume Connection
```

#### WebRTC Implementation
```
Magic Link Click → Open PWA → WebRTC Setup
P2P Connection → Browser Permissions → Audio Stream
Quality Monitor → Adaptive Bitrate → Visual Feedback
```

### Accessibility Wireframe Annotations

#### Screen Reader Navigation Flow
```
Main Screen:
- "Humr Voice Communication, application"
- "Start Voice Chat, button, begins new secure conversation"
- "Join Voice Chat, button, connect to existing conversation"
- "Recent connections, list of 3 items"

Lighthouse Mode:
- "Ready for connection, heading"
- "Room name: sunset-dragon-47"
- "QR code for connection sharing, button, tap to enlarge"
- "Magic link: humr://sunset-dragon-47, text field, tap to copy"
- "Share via messages, button" (etc.)

Voice Chat Active:
- "Connected to Alice Thompson, heading"
- "Your voice level, progress indicator, currently talking"
- "Alice's voice level, progress indicator, currently silent"
- "Mute microphone, toggle button, currently unmuted"
```

#### High Contrast Mode Considerations
```
Color Dependencies → Alternative Indicators:
🟢 Green status → ✓ Checkmark + "Connected"
🟡 Yellow status → ⚠ Warning + "Connecting"
🔴 Red status → ✗ X mark + "Disconnected"
```

### Error State Wireframes

#### Connection Failed State
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Try Again                   Humr                            × Close       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│                           ⚠️ Connection Challenge                           │
│                                                                             │
│    We're having trouble connecting to "sunset-dragon-47"                   │
│                                                                             │
│    Let's try a different approach:                                          │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🔄 Try Again                                                   │ →  │
│    │     Sometimes connections need a moment to establish             │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🌐 Use Internet Connection                                     │ →  │
│    │     Connect through the internet instead of local network       │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  📋 Copy Diagnostic Info                                        │ →  │
│    │     Share technical details with the other person               │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  🆘 Get Help                                                    │ →  │
│    │     View troubleshooting guide and contact support              │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ Don't worry - P2P connections sometimes need patience • We'll figure it out │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Network Quality Warning State
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  sunset-dragon-47          🔊 Volume: 75%               📞 End Chat         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│    ⚠️ Connection Quality Notice                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │  Network conditions have changed. Audio quality has been        │     │
│    │  automatically adjusted to maintain your conversation.          │     │
│    │                                                                 │     │
│    │  Quality: Good → Fair                                           │     │
│    │  Bitrate: 64kbps → 32kbps                                      │     │
│    │                                                                 │     │
│    │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │     │
│    │  │   Dismiss   │  │  Retry High │  │ Show Details│             │     │
│    │  └─────────────┘  └─────────────┘  └─────────────┘             │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│                       🟡 Connected to Alice Thompson                        │
│                             Internet • 89ms latency                         │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                     🎙️ YOU ARE TALKING                          │     │
│    │   ████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░       │     │
│    │                      Your voice level                           │     │
│    │                                                                 │     │
│    │   🟢 Noise suppression active • Echo cancellation on           │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
│    ┌─────────────────────────────────────────────────────────────────┐     │
│    │                    🔊 ALICE IS TALKING                          │     │
│    │   ░░░░░░░░░░████████████████████████████████████░░░░░░░░░░       │     │
│    │                    Alice's voice level                          │     │
│    │                                                                 │     │
│    │   🔐 End-to-end encrypted • 🛡️ Forward secrecy active          │     │
│    └─────────────────────────────────────────────────────────────────┘     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🔐 Secure • 🟡 89ms latency • 📶 Fair • 32kbps • Adapting to network      │
└─────────────────────────────────────────────────────────────────────────────┘
```

This comprehensive wireframe documentation provides the detailed visual and interaction foundation needed to implement the revolutionary "AirDrop for Voice" UX. The wireframes demonstrate how technical complexity is hidden behind intuitive interfaces while maintaining transparency about security and connection quality.